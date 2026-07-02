use std::{
    collections::HashSet,
    env, fs, io,
    process::{ExitCode, exit},
    time::SystemTime,
};

use dvm::{
    frontend::{self, AssembledProgram},
    memory_lane::MemoryLane,
    opcode::Opcode,
    perf_profile::PerfProfiler,
    program::Program,
    vm::Vm,
};

const HELP: &str = "\
Usage:
  dvm --help
  dvm run <program.dvm>
  dvm run [--perf] [--profiling <profile.json>] <program.dvm>
  dvm debug <program.dvm>
";

const DEBUG_HELP: &str = "\
n, next           execute the next opcode
ip                print the current instruction pointer
ops               print all opcodes with their instruction pointers
e, execute OPCODE execute a custom opcode instead of the next one
r, run            run until the next breakpoint should get executed
br, break IP      break at the instruction pointer, call again with the same IP to remove
s, stack          print the current stack
q, quit           quits the debugger
h, help           print this help message
";

const DEBUG_HELPER_STRING: &str = "Please select: n/ip/ops/e/r/br/s/q/h";

fn register_standard_external_functions(vm: &mut Vm<'_>) {
    vm.register_external_function("print", |mut args| {
        println!("{}", args.stack().pop_i32()?);
        Ok(())
    });

    vm.register_external_function("i32.SQRT", |mut args| {
        let value = args.stack().pop_i32()?;

        let root = value.isqrt();
        args.stack().push_i32(root)?;

        Ok(())
    });

    vm.register_external_function("f32.POW", |mut args| {
        let exponent = args.stack().pop_f32()?;
        let base = args.stack().pop_f32()?;
        args.stack().push_f32(base.powf(exponent))?;
        Ok(())
    });

    vm.register_external_function("f32.PRINT", |mut args| {
        println!("{}", args.stack().pop_f32()?);
        Ok(())
    });
}

fn load_opcodes(path: &str) -> Result<Vec<Opcode>, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("could not read program file {path}: {e}"))?;

    frontend::compile_source(path, &source)
}

fn load_program(path: &str) -> Result<AssembledProgram, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("could not read program file {path}: {e}"))?;

    frontend::compile_source_with_metadata(path, &source)
}

struct RunOptions<'a> {
    perf: bool,
    profiling_path: Option<&'a str>,
}

fn run_program(path: &str, options: RunOptions<'_>) -> Result<i32, String> {
    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut vm = Vm::new(Box::new(memory_lanes));
    register_standard_external_functions(&mut vm);

    let AssembledProgram { opcodes, metadata } = load_program(path)?;
    let mut program = Program::new(&opcodes);
    let mut exit_code = 0;

    let start = SystemTime::now();

    loop {
        let step_result = vm
            .step(&mut program)
            .map_err(|e| format!("runtime error: {e}"))?;

        if let Some(finish) = step_result {
            exit_code = finish;
            break;
        }

        if program.is_outside_program() {
            break;
        }
    }

    if options.perf {
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
        eprintln!("IP metrics:");
        eprintln!("Count\tIP\tLine\tOpcode");
        let mut previous_source_line = None;
        for (ip, count) in program.get_line_metrics().iter().enumerate() {
            let source_line = metadata.source_lines_by_ip.get(ip).copied();
            let source_line_display = match source_line {
                Some(line) if Some(line) != previous_source_line => line.to_string(),
                _ => String::new(),
            };
            previous_source_line = source_line;

            eprintln!("{count}\t{ip}\t{source_line_display}\t{:?}", opcodes[ip]);
        }
    }

    if let Some(profile_path) = options.profiling_path {
        let mut profiler = PerfProfiler::new(path, &metadata, &opcodes);
        profiler.record_ip_counts(program.get_line_metrics());
        profiler.write_to_file(profile_path)?;
        eprintln!("Firefox Profiler profile written to {profile_path}");
    }

    Ok(exit_code)
}

fn prompt_debug_command() -> Result<Option<String>, String> {
    println!("{}", DEBUG_HELPER_STRING);

    let mut input = String::new();
    let bytes = io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read stdin: {e}"))?;

    if bytes == 0 {
        return Ok(None);
    }

    Ok(Some(input.trim().to_string()))
}

fn print_opcodes(opcodes: &[Opcode]) {
    for (ip, opcode) in opcodes.iter().enumerate() {
        println!("{ip}\t{:?}", opcode);
    }
}

fn debug_program(path: &str) -> Result<(), String> {
    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut vm = Vm::new(Box::new(memory_lanes));
    register_standard_external_functions(&mut vm);
    let opcodes = load_opcodes(path)?;
    let mut program = Program::new(&opcodes);

    let mut break_points = HashSet::new();
    let mut running = false;

    loop {
        if program.is_outside_program() {
            break;
        }

        let ip = program.get_ip_counter();

        if break_points.contains(&ip) && running {
            running = false;
        } else if running {
            if let Some(status_code) = vm
                .step(&mut program)
                .map_err(|e| format!("runtime error: {e}"))?
            {
                exit(status_code);
            }
        } else {
            let opcode = program
                .get_current_opcode()
                .ok_or_else(|| "instruction pointer out of bounds".to_string())?;

            loop {
                println!("{ip} {:?}", &opcode);

                let Some(command) = prompt_debug_command()? else {
                    return Ok(());
                };

                match command.as_str() {
                    "n" | "next" => {
                        if let Some(status_code) = vm
                            .step(&mut program)
                            .map_err(|e| format!("runtime error: {e}"))?
                        {
                            exit(status_code);
                        }

                        break;
                    }
                    "ip" => {
                        println!("IP: {}", program.get_ip_counter());
                    }
                    "ops" => {
                        print_opcodes(&opcodes);
                    }
                    "s" | "stack" | "stac" => {
                        println!("Stack bytes: {:?}", vm.get_stack_bytes());
                    }
                    "h" | "help" => {
                        print!("{DEBUG_HELP}");
                    }
                    "r" | "run" => {
                        running = true;
                        break;
                    }
                    "q" | "quit" => {
                        exit(1);
                    }
                    _ => {
                        if let Some(rest) = command
                            .strip_prefix("e")
                            .or_else(|| command.strip_prefix("execute"))
                            .map(|item| item.trim())
                        {
                            match serde_plain::from_str::<Opcode>(rest) {
                                Ok(opcode) => {
                                    vm.execute_opcode(&mut program, &opcode)
                                        .map_err(|e| format!("runtime error: {e}"))?;
                                    break;
                                }
                                Err(e) => {
                                    println!("Invalid opcode: {e}");
                                }
                            }
                        } else if let Some(rest) = command
                            .strip_prefix("br")
                            .or_else(|| command.strip_prefix("break"))
                            .map(|item| item.trim())
                        {
                            match serde_plain::from_str::<usize>(rest) {
                                Ok(break_point) => {
                                    if break_points.contains(&break_point) {
                                        break_points.remove(&break_point);
                                        println!("Removed breakpoint at ip={break_point}");
                                    } else {
                                        break_points.insert(break_point);
                                        println!("Set breakpoint at ip={break_point}");
                                    }
                                }
                                Err(e) => {
                                    println!("Invalid ip for breakpoint: {e}")
                                }
                            }
                        } else {
                            println!("Unknown command");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_run_args(args: &[String]) -> Result<(RunOptions<'_>, &str), String> {
    let mut perf = false;
    let mut profiling_path = None;
    let mut program_path = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--perf" => {
                perf = true;
                index += 1;
            }
            "--profiling" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(
                        "usage: dvm run [--perf] [--profiling <profile.json>] <program.dvm>"
                            .to_string(),
                    );
                };
                if profiling_path.replace(path.as_str()).is_some() {
                    return Err("--profiling can only be provided once".to_string());
                }
                index += 2;
            }
            arg if arg.starts_with("--") => {
                return Err(format!(
                    "unknown run option {arg}\n\nusage: dvm run [--perf] [--profiling <profile.json>] <program.dvm>"
                ));
            }
            path => {
                if program_path.replace(path).is_some() {
                    return Err(
                        "usage: dvm run [--perf] [--profiling <profile.json>] <program.dvm>"
                            .to_string(),
                    );
                }
                index += 1;
            }
        }
    }

    let Some(program_path) = program_path else {
        return Err(
            "usage: dvm run [--perf] [--profiling <profile.json>] <program.dvm>".to_string(),
        );
    };

    Ok((
        RunOptions {
            perf,
            profiling_path,
        },
        program_path,
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_run_args;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn parses_plain_run_args() {
        let args = args(&["program.dvm"]);
        let (options, path) = parse_run_args(&args).unwrap();

        assert_eq!(path, "program.dvm");
        assert!(!options.perf);
        assert_eq!(options.profiling_path, None);
    }

    #[test]
    fn parses_perf_without_enabling_profiling() {
        let args = args(&["--perf", "program.dvm"]);
        let (options, path) = parse_run_args(&args).unwrap();

        assert_eq!(path, "program.dvm");
        assert!(options.perf);
        assert_eq!(options.profiling_path, None);
    }

    #[test]
    fn parses_profiling_output_path() {
        let args = args(&["--profiling", "profile.json", "program.dvm"]);
        let (options, path) = parse_run_args(&args).unwrap();

        assert_eq!(path, "program.dvm");
        assert!(!options.perf);
        assert_eq!(options.profiling_path, Some("profile.json"));
    }

    #[test]
    fn parses_perf_with_profiling_output_path() {
        let args = args(&["--perf", "--profiling", "profile.json", "program.dvm"]);
        let (options, path) = parse_run_args(&args).unwrap();

        assert_eq!(path, "program.dvm");
        assert!(options.perf);
        assert_eq!(options.profiling_path, Some("profile.json"));
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
            match parse_run_args(rest).and_then(|(options, path)| run_program(path, options)) {
                Ok(status_code) => exit(status_code),
                Err(err) => Err(err),
            }
        }
        [command, path] if command == "debug" => debug_program(path),
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
