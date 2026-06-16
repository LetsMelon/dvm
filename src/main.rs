use dvm::{
    opcode::{Opcode, SwapDirection},
    program::Program,
};

fn program_word_count() -> Vec<Opcode> {
    vec![
        Opcode::SwitchMemoryLane(0), // switch to heap lane for the in_word flag
        Opcode::Zero,                // push 0
        Opcode::Write(0),            // in_word = 0 at heap[0]
        Opcode::Zero,                // count = 0
        Opcode::SwitchMemoryLane(1), // switch to file lane
        Opcode::SizeOfMemoryLane,    // size = len(file)
        Opcode::Zero,                // index = 0
        Opcode::DupN(2),             // copy index and size for the loop condition
        Opcode::SmallerThan,         // compute index < size
        Opcode::PushIntermediate(4), // delta to continue into the loop body
        Opcode::JumpIfTrue,          // if true continue into the loop body
        Opcode::Pop,                 // else drop index
        Opcode::Pop,                 // else drop size
        Opcode::PushIntermediate(1000), // explicit exit past the program
        Opcode::Jump,                // leave the program
        Opcode::Dup,                 // loop body: copy index so one copy can be consumed by Read
        Opcode::Read,                // byte = file[index]
        Opcode::PushIntermediate(-41), // return delta from is_text_function back to the branch site
        Opcode::Swap(SwapDirection::Right, 2), // keep the return delta below the original byte
        Opcode::Dup,                 // duplicate byte for the space check
        Opcode::PushIntermediate(' ' as i64), // push ' '
        Opcode::Xor,                 // byte ^ ' '
        Opcode::Not,                 // check byte == ' '
        Opcode::Swap(SwapDirection::Right, 2), // move the byte back to the top for the next check
        Opcode::Dup,                 // duplicate byte for the comma check
        Opcode::PushIntermediate(',' as i64), // push ','
        Opcode::Xor,                 // byte ^ ','
        Opcode::Not,                 // check byte == ','
        Opcode::Swap(SwapDirection::Right, 2), // move the byte back to the top for the next check
        Opcode::Dup,                 // duplicate byte for the exclamation check
        Opcode::PushIntermediate('!' as i64), // push '!'
        Opcode::Xor,                 // byte ^ '!'
        Opcode::Not,                 // check byte == '!'
        Opcode::Swap(SwapDirection::Right, 2), // move the byte back to the top for the next check
        Opcode::Dup,                 // duplicate byte for the dot check
        Opcode::PushIntermediate('.' as i64), // push '.'
        Opcode::Xor,                 // byte ^ '.'
        Opcode::Not,                 // check byte == '.'
        Opcode::Swap(SwapDirection::Right, 2), // move the byte back to the top for the next check
        Opcode::Dup,                 // duplicate byte for the newline check
        Opcode::PushIntermediate('\n' as i64), // push newline
        Opcode::Xor,                 // byte ^ '\n'
        Opcode::Not,                 // check byte == '\n'
        Opcode::Swap(SwapDirection::Right, 2), // move the byte back to the top for the next check
        Opcode::Dup,                 // duplicate byte for the carriage-return check
        Opcode::PushIntermediate('\r' as i64), // push carriage return
        Opcode::Xor,                 // byte ^ '\r'
        Opcode::Not,                 // check byte == '\r'
        Opcode::Swap(SwapDirection::Right, 2), // move the byte back to the top for the next check
        Opcode::PushIntermediate('\t' as i64), // push tab for the final check
        Opcode::Xor,                 // byte ^ '\t'
        Opcode::Not,                 // check byte == '\t'
        Opcode::PushIntermediate(32), // delta to is_text_function
        Opcode::Jump,                // call is_text_function
        Opcode::PushIntermediate(8), // delta to the text handler
        Opcode::JumpIfTrue,          // if is_text jump to the text handler
        Opcode::SwitchMemoryLane(0), // separator: switch to heap lane
        Opcode::Zero,                // push 0
        Opcode::Write(0),            // in_word = 0 at heap[0]
        Opcode::SwitchMemoryLane(1), // switch back to file lane
        Opcode::PushIntermediate(1), // push 1
        Opcode::Add,                 // index += 1
        Opcode::PushIntermediate(-57), // delta back to the loop condition
        Opcode::Jump,                // loop
        Opcode::SwitchMemoryLane(0), // text: switch to heap lane
        Opcode::Zero,                // push address 0
        Opcode::Read,                // load in_word from heap[0]
        Opcode::Not,                 // check in_word == 0
        Opcode::PushIntermediate(5), // delta to start_new_word
        Opcode::JumpIfTrue,          // if not already in a word, jump to start_new_word
        Opcode::SwitchMemoryLane(1), // continue current word: switch back to file lane
        Opcode::PushIntermediate(1), // push 1
        Opcode::Add,                 // index += 1
        Opcode::PushIntermediate(-68), // delta back to the loop condition
        Opcode::Jump,                // loop
        Opcode::Swap(SwapDirection::Left, 3), // start_new_word: rotate [count, size, index] to [size, index, count]
        Opcode::PushIntermediate(1),          // push 1
        Opcode::Add,                          // count += 1
        Opcode::Swap(SwapDirection::Right, 3), // rotate back to [count, size, index]
        Opcode::PushIntermediate(1),          // push 1
        Opcode::Write(0),                     // in_word = 1 at heap[0]
        Opcode::SwitchMemoryLane(1),          // switch back to file lane
        Opcode::PushIntermediate(1),          // push 1
        Opcode::Add,                          // index += 1
        Opcode::PushIntermediate(-79),        // delta back to the loop condition
        Opcode::Jump,                         // loop
        Opcode::Or,                           // (c == '\r') || (c == '\t')
        Opcode::Or,                           // accumulate separator matches
        Opcode::Or,                           // accumulate separator matches
        Opcode::Or,                           // accumulate separator matches
        Opcode::Or,                           // accumulate separator matches
        Opcode::Or,                           // current byte is a separator
        Opcode::Not,                          // result: current byte is text
        Opcode::Swap(SwapDirection::Left, 2), // move the saved return delta to the top
        Opcode::Jump,                         // return with is_text left on the stack
    ]
}

fn print_program(program: &Program) {
    for opcode in program.get_opcodes() {
        println!(
            "{}",
            serde_plain::to_string(&opcode)
                .expect(&format!("Could not deserialize opcode={:?}", opcode))
        );
    }
}

fn main() {
    use std::time::SystemTime;

    use dvm::{memory_lane::MemoryLane, program::Program, vm::Vm};

    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut vm = Vm::new(Box::new(memory_lanes));
    let opcodes = program_word_count();
    let mut program = Program::new(&opcodes);

    print_program(&program);

    let start = SystemTime::now();

    loop {
        let step_result = vm.step(&mut program);

        if let Err(e) = step_result {
            eprintln!("Error: {}", e);
            break;
        }

        if let Ok(finish) = step_result {
            if finish {
                break;
            }
        }

        if program.is_outside_program() {
            break;
        }
    }

    let end = SystemTime::now();
    let duration = end
        .duration_since(start)
        .unwrap_or(std::time::Duration::from_secs(0));

    println!(
        "\nExecution finished after {} operations",
        vm.get_op_counter()
    );
    println!("Execution time: {:?}", duration);
    println!(
        "Operations per second: {:.2}",
        vm.get_op_counter() as f64 / duration.as_secs_f64()
    );

    println!("\nLine metrics:");
    println!("Count\tLine\tOpcode");
    for (i, count) in program.get_line_metrics().iter().enumerate() {
        println!("{}\t{:?}\t{:?}", count, i, opcodes[i]);
    }
}
