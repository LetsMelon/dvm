use std::{
    collections::HashSet,
    env, fs, io,
    process::{ExitCode, exit},
    time::SystemTime,
};

use dvm::{memory_lane::MemoryLane, opcode::Opcode, program::Program, vm::Vm};

const HELP: &str = "\
Usage:
  dvm --help
  dvm run <program.dvm>
  dvm run --perf <program.dvm>
  dvm debug <program.dvm>
";

const DEBUG_HELP: &str = "\
n, next           execute the next opcode
ip                print the current instruction pointer
e, execute OPCODE execute a custom opcode instead of the next one
r, run            run until the next breakpoint should get executed
br, break IP      break at the instruction pointer, call again with the same IP to remove
s, stack          print the current stack
q, quit           quits the debugger
h, help           print this help message
";

const DEBUG_HELPER_STRING: &str = "Please select: n/ip/e/r/br/s/q/h";

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

fn run_program(path: &str, perf: bool) -> Result<i32, String> {
    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut vm = Vm::new(Box::new(memory_lanes));
    let opcodes = load_opcodes(path)?;
    let mut program = Program::new(&opcodes);
    let mut exit_code = 0;

    let start = SystemTime::now();

    loop {
        let step_result = vm.step(&mut program);

        if let Err(e) = step_result {
            return Err(format!("runtime error: {e}"));
        }

        if let Ok(Some(finish)) = step_result {
            exit_code = finish;
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

fn debug_program(path: &str) -> Result<(), String> {
    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut vm = Vm::new(Box::new(memory_lanes));
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
                    "s" | "stack" | "stac" => {
                        println!("Stack: {:?}", vm.get_stack());
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
        [command, rest @ ..] if command == "run" => match parse_run_args(rest)
            .and_then(|(perf, path)| run_program(path, perf))
        {
            Ok(status_code) => exit(status_code),
            Err(err) => Err(err),
        },
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
