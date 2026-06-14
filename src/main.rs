// Stack based
// SP counter
// Memory lanes

use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
enum SwapDirection {
    Left,
    Right,
}

#[derive(Debug)]
enum Opcode {
    // Push(i8),
    // PushI16(i16),
    // PushI32(i32),
    // PushI64(i64),
    /// Pops an address from the stack, reads one byte from the current memory lane, and pushes it
    Read,
    /// Writes the top byte from the stack to the given address in the current memory lane
    Write(u64),
    /// Switches the memory lane to the given lane
    SwitchMemoryLane(u8),
    SizeOfMemoryLane,
    Noop,
    Pop,
    Pop32bits,
    Dup,
    DupN(u8),
    Swap(SwapDirection, u8),
    Xor,
    Zero,
    PushImmediate(i64),
    Add,
    SmallerThan,
    /// Jump to the opcode at the given delta if the top of the stack is non-zero
    JumpIfTrue(i32),
    Not,
    Print,
    PrintN(u8),
}

enum MemoryLane<'a> {
    ReadOnly(&'a [u8]),
    ReadWrite(&'a mut [u8]),
}

impl<'a> MemoryLane<'a> {
    fn size(&self) -> u64 {
        match self {
            MemoryLane::ReadOnly(slice) => slice.len() as u64,
            MemoryLane::ReadWrite(slice) => slice.len() as u64,
        }
    }
}

fn main() {
    let mut heap_memory_lane = [0; 1024];
    let io_memory_lane = include_bytes!("../test.txt");

    let mut memory_lanes = [
        MemoryLane::ReadWrite(&mut heap_memory_lane),
        MemoryLane::ReadOnly(io_memory_lane),
    ];

    let mut ip_counter = 0;
    let mut current_memory_lane = 0;
    let mut stack = Vec::new();
    let mut op_counter = 0;

    let opcodes = [
        Opcode::SwitchMemoryLane(0), // switch to heap lane for the in_word flag
        Opcode::Zero,                // push 0
        Opcode::Write(0),            // in_word = 0 at heap[0]
        Opcode::Zero,                // count = 0
        Opcode::SwitchMemoryLane(1), // switch to file lane
        Opcode::SizeOfMemoryLane,    // size = len(file)
        Opcode::Zero,                // index = 0
        Opcode::DupN(2),             // copy index and size for the loop condition
        Opcode::SmallerThan,         // compute index < size
        Opcode::JumpIfTrue(4),       // if true continue into the loop body
        Opcode::Pop,                 // else drop index
        Opcode::Pop,                 // else drop size
        Opcode::PushImmediate(1),    // push true to force an exit jump
        Opcode::JumpIfTrue(68),      // jump past the loop body
        Opcode::Dup,                 // copy index so one copy can be consumed by Read
        Opcode::Read,                // byte = file[index]
        Opcode::Dup,                 // duplicate byte for the space check
        Opcode::PushImmediate(' ' as i64), // push ' '
        Opcode::Xor,                 // byte ^ ' '
        Opcode::Not,                 // check byte == ' '
        Opcode::JumpIfTrue(52),      // if separator jump to the separator handler
        Opcode::Dup,                 // duplicate byte for the comma check
        Opcode::PushImmediate(',' as i64), // push ','
        Opcode::Xor,                 // byte ^ ','
        Opcode::Not,                 // check byte == ','
        Opcode::JumpIfTrue(47),      // if separator jump to the separator handler
        Opcode::Dup,                 // duplicate byte for the exclamation check
        Opcode::PushImmediate('!' as i64), // push '!'
        Opcode::Xor,                 // byte ^ '!'
        Opcode::Not,                 // check byte == '!'
        Opcode::JumpIfTrue(42),      // if separator jump to the separator handler
        Opcode::Dup,                 // duplicate byte for the dot check
        Opcode::PushImmediate('.' as i64), // push '.'
        Opcode::Xor,                 // byte ^ '.'
        Opcode::Not,                 // check byte == '.'
        Opcode::JumpIfTrue(37),      // if separator jump to the separator handler
        Opcode::Dup,                 // duplicate byte for the newline check
        Opcode::PushImmediate('\n' as i64), // push newline
        Opcode::Xor,                 // byte ^ '\n'
        Opcode::Not,                 // check byte == '\n'
        Opcode::JumpIfTrue(32),      // if separator jump to the separator handler
        Opcode::Dup,                 // duplicate byte for the carriage-return check
        Opcode::PushImmediate('\r' as i64), // push carriage return
        Opcode::Xor,                 // byte ^ '\r'
        Opcode::Not,                 // check byte == '\r'
        Opcode::JumpIfTrue(27),      // if separator jump to the separator handler
        Opcode::Dup,                 // duplicate byte for the tab check
        Opcode::PushImmediate('\t' as i64), // push tab
        Opcode::Xor,                 // byte ^ '\t'
        Opcode::Not,                 // check byte == '\t'
        Opcode::JumpIfTrue(22),      // if separator jump to the separator handler
        Opcode::Pop,                 // non-separator: drop byte
        Opcode::SwitchMemoryLane(0), // switch to heap lane
        Opcode::Zero,                // push address 0
        Opcode::Read,                // load in_word from heap[0]
        Opcode::Not,                 // check in_word == 0
        Opcode::JumpIfTrue(5),       // if not already in a word, jump to start_new_word
        Opcode::SwitchMemoryLane(1), // continue current word: switch back to file lane
        Opcode::PushImmediate(1),    // push 1
        Opcode::Add,                 // index += 1
        Opcode::PushImmediate(1),    // push true to force the loop jump
        Opcode::JumpIfTrue(-55),     // jump back to the loop condition
        Opcode::Swap(SwapDirection::Left, 3), // start_new_word: rotate [count, size, index] to [size, index, count]
        Opcode::PushImmediate(1),             // push 1
        Opcode::Add,                          // count += 1
        Opcode::Swap(SwapDirection::Right, 3), // rotate back to [count, size, index]
        Opcode::PushImmediate(1),             // push 1
        Opcode::Write(0),                     // in_word = 1 at heap[0]
        Opcode::SwitchMemoryLane(1),          // switch back to file lane
        Opcode::PushImmediate(1),             // push 1
        Opcode::Add,                          // index += 1
        Opcode::PushImmediate(1),             // push true to force the loop jump
        Opcode::JumpIfTrue(-66),              // jump back to the loop condition
        Opcode::Pop,                          // separator: drop byte
        Opcode::SwitchMemoryLane(0),          // switch to heap lane
        Opcode::Zero,                         // push 0
        Opcode::Write(0),                     // in_word = 0 at heap[0]
        Opcode::SwitchMemoryLane(1),          // switch back to file lane
        Opcode::PushImmediate(1),             // push 1
        Opcode::Add,                          // index += 1
        Opcode::PushImmediate(1),             // push true to force the loop jump
        Opcode::JumpIfTrue(-75),              // jump back to the loop condition
    ];

    let start = SystemTime::now();

    loop {
        op_counter += 1;

        let out = step(
            &opcodes,
            &mut ip_counter,
            &mut current_memory_lane,
            &mut stack,
            &mut memory_lanes,
        );

        if let Err(e) = out {
            eprintln!("Error: {}", e);
            break;
        }

        if ip_counter >= opcodes.len() {
            break;
        }

        if let Ok(finish) = out {
            if finish {
                break;
            }
        }
    }

    let end = SystemTime::now();
    let duration = end
        .duration_since(start)
        .unwrap_or(std::time::Duration::from_secs(0));

    println!("Word count: {}", stack.last().copied().unwrap_or(0));
    println!("\nExecution finished after {} operations", op_counter);
    println!("Execution time: {:?}", duration);
    println!(
        "Operations per second: {:.2}",
        op_counter as f64 / duration.as_secs_f64()
    );
}

fn pop_u64(stack: &mut Vec<u64>) -> Result<u64, String> {
    stack.pop().ok_or("Stack underflow".into())
}

fn rotate_top(stack: &mut [u64], direction: SwapDirection) {
    match direction {
        SwapDirection::Left => stack.rotate_left(1),
        SwapDirection::Right => stack.rotate_right(1),
    }
}

fn step(
    program: &[Opcode],
    ip_counter: &mut usize,
    current_memory_lane: &mut u8,
    stack: &mut Vec<u64>,
    memory_lanes: &mut [MemoryLane],
) -> Result<bool, String> {
    let opcode = program
        .get(*ip_counter)
        .ok_or("Instruction pointer out of bounds")?;

    // println!(
    //     "IP: {}, Opcode: {:?}, Current Memory Lane: {}, Stack: {:?}",
    //     *ip_counter, opcode, current_memory_lane, stack
    // );

    let memory_lane = memory_lanes
        .get_mut(*current_memory_lane as usize)
        .ok_or("Invalid memory lane")?;

    match opcode {
        Opcode::Read => {
            let address = pop_u64(stack)? as usize;
            let value = match memory_lane {
                MemoryLane::ReadOnly(slice) => {
                    *slice.get(address).ok_or("Read address out of bounds")?
                }
                MemoryLane::ReadWrite(slice) => {
                    *slice.get(address).ok_or("Read address out of bounds")?
                }
            };

            stack.push(value as u64);
            *ip_counter += 1;
        }
        Opcode::Write(address) => {
            let value = pop_u64(stack)?;
            if let MemoryLane::ReadWrite(slice) = memory_lane {
                let value_byte = (value & 0xFF) as u8;
                slice[*address as usize] = value_byte;
            } else {
                return Err("Cannot write to read-only memory lane".into());
            }
            *ip_counter += 1;
        }
        Opcode::SwitchMemoryLane(lane) => {
            *current_memory_lane = *lane;
            *ip_counter += 1;
        }
        Opcode::SizeOfMemoryLane => {
            stack.push(memory_lane.size());
            *ip_counter += 1;
        }
        Opcode::Noop => {
            *ip_counter += 1;
        }
        Opcode::Pop => {
            let _ = pop_u64(stack)?;
            *ip_counter += 1;
        }
        Opcode::Pop32bits => {
            for _ in 0..4 {
                let _ = pop_u64(stack)?;
            }
            *ip_counter += 1;
        }
        Opcode::Dup => {
            let value = *stack.last().ok_or("Stack underflow")?;
            stack.push(value);
            *ip_counter += 1;
        }
        Opcode::DupN(n) => {
            let n = *n as usize;
            if n == 0 {
                return Err("DupN requires n >= 1".into());
            }

            let len = stack.len();
            if n > len {
                return Err("Stack underflow".into());
            }

            for i in 0..n {
                let value = stack[len - n + i];
                stack.push(value);
            }

            *ip_counter += 1;
        }
        Opcode::Swap(direction, n) => {
            let n = *n as usize;
            if n < 2 {
                return Err("Swap requires n >= 2".into());
            }

            let len = stack.len();
            if n > len {
                return Err("Stack underflow".into());
            }

            rotate_top(&mut stack[len - n..], *direction);
            *ip_counter += 1;
        }
        Opcode::Xor => {
            let a = pop_u64(stack)?;
            let b = pop_u64(stack)?;
            stack.push(a ^ b);
            *ip_counter += 1;
        }
        Opcode::Zero => {
            stack.push(0);
            *ip_counter += 1;
        }
        Opcode::PushImmediate(value) => {
            stack.push(*value as u64);
            *ip_counter += 1;
        }
        Opcode::Add => {
            let a = pop_u64(stack)?;
            let b = pop_u64(stack)?;
            stack.push(a.wrapping_add(b));
            *ip_counter += 1;
        }
        Opcode::SmallerThan => {
            let a = pop_u64(stack)?;
            let b = pop_u64(stack)?;
            stack.push((a < b) as u64);
            *ip_counter += 1;
        }
        Opcode::JumpIfTrue(delta_address) => {
            let condition = pop_u64(stack)?;
            if condition != 0 {
                // TODO add check if we have have a ip counter that is negative
                *ip_counter = ((*ip_counter as i64) + (*delta_address as i64) + 1) as usize;
            } else {
                *ip_counter += 1;
            }
        }
        Opcode::Not => {
            let value = pop_u64(stack)?;
            stack.push((value == 0) as u64);
            *ip_counter += 1;
        }
        Opcode::Print => {
            let value = pop_u64(stack)?;
            print!("{}", value as u8 as char);
            *ip_counter += 1;
        }
        Opcode::PrintN(size) => {
            let size = *size as usize;
            let len = stack.len();
            if size > len {
                return Err("Stack underflow".into());
            }

            for i in 0..size {
                let value = stack[len - size + i];
                print!("{}", value as u8 as char);
            }
            stack.truncate(stack.len() - size);

            *ip_counter += 1;
        }
    }

    Ok(false)
}
