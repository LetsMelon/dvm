use std::{env, fs, process::ExitCode, time::SystemTime};

use dvm::{memory_lane::MemoryLane, opcode::Opcode, program::Program, vm::Vm};

const HELP: &str = "\
Usage:
  dvm --help
  dvm run <program.dvm>
  dvm run --perf <program.dvm>
  dvm debug <program.dvm>
";

fn load_opcodes(path: &str) -> Result<Vec<Opcode>, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("could not read program file {path}: {e}"))?;

    let mut opcodes = Vec::new();
    for (line_no, raw_line) in source.lines().enumerate() {
        let mut line = raw_line;
        if let Some(comment_idx) = line.find("//") {
            line = &line[0..comment_idx];
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let opcode = serde_plain::from_str::<Opcode>(line)
            .map_err(|e| format!("{path}:{}: {e}", line_no + 1))?;
        opcodes.push(opcode);
    }

    if opcodes.is_empty() {
        return Err(format!("program file {path} did not contain any opcodes"));
    }

    Ok(opcodes)
}

fn run_program(path: &str, perf: bool) -> Result<(), String> {
    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut vm = Vm::new(Box::new(memory_lanes));
    let opcodes = load_opcodes(path)?;
    let mut program = Program::new(&opcodes);

    let start = SystemTime::now();

    loop {
        let step_result = vm.step(&mut program);

        if let Err(e) = step_result {
            return Err(format!("runtime error: {e}"));
        }

        if let Ok(finish) = step_result
            && finish
        {
            break;
        }

        if program.is_outside_program() {
            break;
        }
    }

    if perf {
        let duration = SystemTime::now()
            .duration_since(start)
            .unwrap_or(std::time::Duration::from_secs(0));

        eprintln!(
            "Execution finished after {} operations",
            vm.get_op_counter()
        );
        eprintln!("Execution time: {:?}", duration);
        eprintln!(
            "Operations per second: {:.2}",
            vm.get_op_counter() as f64 / duration.as_secs_f64()
        );
        eprintln!();
        eprintln!("Line metrics:");
        eprintln!("Count\tLine\tOpcode");
        for (i, count) in program.get_line_metrics().iter().enumerate() {
            eprintln!("{}\t{}\t{:?}", count, i, opcodes[i]);
        }
    }

    Ok(())
}

fn parse_run_args(args: &[String]) -> Result<(bool, &str), String> {
    match args {
        [path] => Ok((false, path.as_str())),
        [flag, path] if flag == "--perf" => Ok((true, path.as_str())),
        _ => Err("usage: dvm run [--perf] <program.dvm>".to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let result = match args.as_slice() {
        [] => {
            print!("{HELP}");
            Ok(())
        }
        [arg] if arg == "--help" || arg == "-h" => {
            print!("{HELP}");
            Ok(())
        }
        [command, rest @ ..] if command == "run" => {
            parse_run_args(rest).and_then(|(perf, path)| run_program(path, perf))
        }
        [command, path] if command == "debug" => {
            let _ = path;
            todo!("debug CLI not implemented yet")
        }
        _ => Err(format!("unknown arguments\n\n{HELP}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
