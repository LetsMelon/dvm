// Stack based
// SP counter
// Memory lanes

use std::time::SystemTime;

#[derive(Debug)]
enum Opcode {
    // Push(i8),
    // PushI16(i16),
    // PushI32(i32),
    // PushI64(i64),
    /// Reads the byte at the given address and writes it onto the selected memory lane
    Read(u64),
    /// Writes the top byte from the stack to the given address in the current memory lane
    Write(u64),
    /// Switches the memory lane to the given lane
    SwitchMemoryLane(u8),
    SizeOfMemoryLane,
    Noop,
    Pop,
    Pop32bits,
    Copy,
    CopyN(u8),
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
        Opcode::SwitchMemoryLane(1),
        Opcode::SizeOfMemoryLane,
        Opcode::SwitchMemoryLane(0),
        Opcode::Zero,
        Opcode::Noop,
        Opcode::PushImmediate(1),
        Opcode::Add,
        Opcode::Noop,
        Opcode::PushImmediate('i' as i64),
        Opcode::PushImmediate(' ' as i64),
        Opcode::PushImmediate('=' as i64),
        Opcode::PushImmediate(' ' as i64),
        Opcode::PushImmediate('\n' as i64),
        Opcode::PrintN(5),
        Opcode::Noop,
        Opcode::CopyN(2),
        Opcode::Noop,
        Opcode::SmallerThan,
        Opcode::Noop,
        Opcode::JumpIfTrue(-15),
        Opcode::Noop,
        Opcode::PushImmediate('H' as i64),
        Opcode::PushImmediate('e' as i64),
        Opcode::PushImmediate('l' as i64),
        Opcode::PushImmediate('l' as i64),
        Opcode::PushImmediate('o' as i64),
        Opcode::PushImmediate(' ' as i64),
        Opcode::PushImmediate('W' as i64),
        Opcode::PushImmediate('o' as i64),
        Opcode::PushImmediate('r' as i64),
        Opcode::PushImmediate('l' as i64),
        Opcode::PushImmediate('d' as i64),
        Opcode::PushImmediate('!' as i64),
        Opcode::PushImmediate('\n' as i64),
        Opcode::PrintN(13),
        Opcode::Noop,
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
        Opcode::Read(_value) => todo!(),
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
        Opcode::Copy => {
            let value = pop_u64(stack)?;
            stack.push(value);
            *ip_counter += 1;
        }
        Opcode::CopyN(n) => {
            let n = *n as usize;

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
