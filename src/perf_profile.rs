use std::{fs::File, io::BufWriter, time::SystemTime};

use fxprof_processed_profile::{
    CategoryColor, CategoryPairHandle, CpuDelta, Frame, FrameFlags, FrameInfo, Profile,
    SamplingInterval, Timestamp,
};

use crate::{frontend::ProgramMetadata, opcode::Opcode};

const MAX_PROFILE_WEIGHT: u64 = 25_000_000;

pub struct PerfProfiler<'a> {
    path: &'a str,
    opcode_frames: Vec<String>,
    line_frames: Vec<String>,
    profile: Profile,
    thread: fxprof_processed_profile::ThreadHandle,
    program_category: CategoryPairHandle,
    line_category: CategoryPairHandle,
    opcode_category: CategoryPairHandle,
    sample_index: u64,
}

impl<'a> PerfProfiler<'a> {
    pub fn new(path: &'a str, metadata: &ProgramMetadata, opcodes: &[Opcode]) -> Self {
        let mut profile = Profile::new(
            "dvm",
            SystemTime::now().into(),
            SamplingInterval::from_millis(1),
        );
        profile.set_symbolicated(true);
        let program_category = profile
            .add_category("VM Program", CategoryColor::Blue)
            .into();
        let line_category = profile
            .add_category("VM Source Line", CategoryColor::Purple)
            .into();
        let opcode_category = profile
            .add_category("VM Opcode", CategoryColor::Green)
            .into();
        let process = profile.add_process(
            "dvm",
            std::process::id(),
            Timestamp::from_millis_since_reference(0.0),
        );
        let thread = profile.add_thread(
            process,
            std::process::id(),
            Timestamp::from_millis_since_reference(0.0),
            true,
        );
        profile.set_thread_name(thread, "VM main thread");
        profile.add_initial_selected_thread(thread);
        profile.add_initial_visible_thread(thread);
        let opcode_frames = opcodes
            .iter()
            .enumerate()
            .map(|(ip, opcode)| opcode_frame_name(metadata, ip, opcode))
            .collect();
        let line_frames = opcodes
            .iter()
            .enumerate()
            .map(|(ip, _)| source_line_frame_name(metadata, ip))
            .collect();

        Self {
            path,
            opcode_frames,
            line_frames,
            profile,
            thread,
            program_category,
            line_category,
            opcode_category,
            sample_index: 0,
        }
    }

    pub fn record_ip_counts(&mut self, counts_by_ip: &[u64]) -> u64 {
        let divisor = profile_weight_divisor(counts_by_ip);

        for (ip, count) in counts_by_ip.iter().copied().enumerate() {
            if count == 0 {
                continue;
            }

            let Some(weight) = scaled_weight(count, divisor) else {
                continue;
            };

            let Some(opcode_frame) = self.opcode_frames.get(ip).cloned() else {
                continue;
            };
            let Some(line_frame) = self.line_frames.get(ip).cloned() else {
                continue;
            };

            self.record_weighted_ip(&line_frame, &opcode_frame, weight);
        }

        divisor
    }

    fn record_weighted_ip(&mut self, line_frame: &str, opcode_frame: &str, weight: i32) {
        let mut stack = Vec::with_capacity(3);
        stack.push(self.frame(self.path, self.program_category));
        stack.push(self.frame(line_frame, self.line_category));
        stack.push(self.frame(opcode_frame, self.opcode_category));

        let stack = self
            .profile
            .intern_stack_frames(self.thread, stack.into_iter());
        self.profile.add_sample(
            self.thread,
            Timestamp::from_millis_since_reference(self.sample_index as f64),
            stack,
            CpuDelta::ZERO,
            weight,
        );
        self.sample_index += 1;
    }

    pub fn write_to_file(self, path: &str) -> Result<(), String> {
        let file = File::create(path)
            .map_err(|e| format!("could not create Firefox profile file {path}: {e}"))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self.profile)
            .map_err(|e| format!("could not write Firefox profile file {path}: {e}"))
    }

    fn frame(&mut self, name: &str, category_pair: CategoryPairHandle) -> FrameInfo {
        FrameInfo {
            frame: Frame::Label(self.profile.intern_string(name)),
            category_pair,
            flags: FrameFlags::IS_JS | FrameFlags::IS_RELEVANT_FOR_JS,
        }
    }
}

fn profile_weight_divisor(counts_by_ip: &[u64]) -> u64 {
    let total_count = counts_by_ip.iter().copied().sum::<u64>();
    total_count.div_ceil(MAX_PROFILE_WEIGHT).max(1)
}

fn scaled_weight(count: u64, divisor: u64) -> Option<i32> {
    if count.saturating_mul(4) <= divisor {
        return None;
    }

    let scaled = count.div_ceil(divisor);
    Some(scaled.min(i32::MAX as u64) as i32)
}

fn opcode_frame_name(metadata: &ProgramMetadata, ip: usize, opcode: &Opcode) -> String {
    let source_line = metadata.source_lines_by_ip.get(ip).copied().unwrap_or(0);
    if source_line == 0 {
        format!("ip {ip:04}: {opcode}")
    } else {
        format!("ip {ip:04}: {opcode} (line {source_line})")
    }
}

fn source_line_frame_name(metadata: &ProgramMetadata, ip: usize) -> String {
    let source_line = metadata.source_lines_by_ip.get(ip).copied().unwrap_or(0);
    if source_line == 0 {
        "line unknown".to_string()
    } else {
        format!("line {source_line}")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{frontend::ProgramMetadata, opcode::Opcode};

    use super::{MAX_PROFILE_WEIGHT, PerfProfiler};

    #[test]
    fn profile_serializes_to_firefox_profile_json() {
        let metadata = ProgramMetadata {
            labels_by_ip: vec![vec![".main".to_string()]],
            source_lines_by_ip: vec![1],
        };
        let opcodes = vec![Opcode::Halt];
        let mut profiler = PerfProfiler::new("test.dvm", &metadata, &opcodes);
        profiler.record_ip_counts(&[1]);

        let json = serde_json::to_value(&profiler.profile).unwrap();
        assert!(matches!(json, Value::Object(_)));
        assert!(json.get("meta").is_some());
        assert!(json.get("threads").is_some() || json.get("processes").is_some());
    }

    #[test]
    fn opcode_frames_are_named_like_functions() {
        let metadata = ProgramMetadata {
            labels_by_ip: vec![Vec::new()],
            source_lines_by_ip: vec![12],
        };
        let opcodes = vec![Opcode::I32Add];
        let mut profiler = PerfProfiler::new("test.dvm", &metadata, &opcodes);
        profiler.record_ip_counts(&[1]);

        let json = serde_json::to_value(&profiler.profile).unwrap();
        let thread = &json["threads"][0];
        let strings = thread["stringArray"].as_array().unwrap();
        assert!(
            strings
                .iter()
                .any(|value| value == "ip 0000: i32.ADD (line 12)")
        );
    }

    #[test]
    fn sample_stack_groups_opcode_under_source_line() {
        let metadata = ProgramMetadata {
            labels_by_ip: vec![Vec::new(), Vec::new(), Vec::new()],
            source_lines_by_ip: vec![1, 2, 3],
        };
        let opcodes = vec![Opcode::I32Zero, Opcode::Dup, Opcode::I32Add];
        let mut profiler = PerfProfiler::new("test.dvm", &metadata, &opcodes);
        profiler.record_ip_counts(&[0, 0, 1]);

        let json = serde_json::to_value(&profiler.profile).unwrap();
        let thread = &json["threads"][0];
        let stack_length = thread["stackTable"]["length"].as_u64().unwrap();
        assert_eq!(stack_length, 3);

        let strings = thread["stringArray"].as_array().unwrap();
        assert!(strings.iter().any(|value| value == "line 3"));
        assert!(
            strings
                .iter()
                .any(|value| value == "ip 0002: i32.ADD (line 3)")
        );
    }

    #[test]
    fn large_counts_are_scaled_to_bounded_profile_weight() {
        let metadata = ProgramMetadata {
            labels_by_ip: vec![Vec::new()],
            source_lines_by_ip: vec![1],
        };
        let opcodes = vec![Opcode::Noop];
        let mut profiler = PerfProfiler::new("test.dvm", &metadata, &opcodes);
        let divisor = profiler.record_ip_counts(&[10_000_000]);

        let json = serde_json::to_value(&profiler.profile).unwrap();
        let thread = &json["threads"][0];
        let weights = thread["samples"]["weight"].as_array().unwrap();

        assert_eq!(divisor, 100);
        assert_eq!(weights.len(), 1);
        assert_eq!(weights[0], MAX_PROFILE_WEIGHT);
    }

    #[test]
    fn scaled_profile_drops_entries_at_or_below_quarter_weight() {
        let metadata = ProgramMetadata {
            labels_by_ip: vec![Vec::new(), Vec::new(), Vec::new()],
            source_lines_by_ip: vec![1, 2, 3],
        };
        let opcodes = vec![Opcode::Noop, Opcode::Dup, Opcode::Halt];
        let mut profiler = PerfProfiler::new("test.dvm", &metadata, &opcodes);
        let divisor = profiler.record_ip_counts(&[9_999_949, 25, 26]);

        let json = serde_json::to_value(&profiler.profile).unwrap();
        let thread = &json["threads"][0];
        let strings = thread["stringArray"].as_array().unwrap();

        assert_eq!(divisor, 100);
        assert!(
            strings
                .iter()
                .any(|value| value == "ip 0000: Noop (line 1)")
        );
        assert!(!strings.iter().any(|value| value == "ip 0001: Dup (line 2)"));
        assert!(
            strings
                .iter()
                .any(|value| value == "ip 0002: Halt (line 3)")
        );
    }
}
