use crate::{
    memory_lane::MemoryLane,
    opcode::{Opcode, SwapDirection},
    program::Program,
};

pub struct Vm<'a> {
    op_counter: u64,
    stack: Vec<i64>,
    memory_lanes: Box<[MemoryLane<'a>]>,
}

impl<'a> Vm<'a> {
    pub fn new(memory_lanes: Box<[MemoryLane<'a>]>) -> Self {
        Vm {
            op_counter: 0,
            stack: Vec::with_capacity(128),
            memory_lanes,
        }
    }

    pub fn step<'b>(&mut self, program: &mut Program<'b>) -> Result<bool, String> {
        self.op_counter += 1;

        step(
            program.opcodes,
            &mut program.ip_counter,
            &mut program.current_memory_lane,
            &mut self.stack,
            &mut self.memory_lanes,
            &mut program.line_metrics,
        )
    }

    pub fn get_op_counter(&self) -> u64 {
        self.op_counter
    }
}

fn pop_i64(stack: &mut Vec<i64>) -> Result<i64, String> {
    stack.pop().ok_or("Stack underflow".into())
}

#[inline]
fn rotate_top(stack: &mut [i64], direction: SwapDirection) {
    match direction {
        SwapDirection::Left => stack.rotate_left(1),
        SwapDirection::Right => stack.rotate_right(1),
    }
}

#[inline]
fn step(
    program: &[Opcode],
    ip_counter: &mut usize,
    current_memory_lane: &mut u8,
    stack: &mut Vec<i64>,
    memory_lanes: &mut [MemoryLane],
    line_metrics: &mut [u64],
) -> Result<bool, String> {
    let opcode = program
        .get(*ip_counter)
        .ok_or("Instruction pointer out of bounds")?;

    line_metrics[*ip_counter] += 1;

    // println!(
    //     "IP: {}, Opcode: {:?}, Current Memory Lane: {}, Stack: {:?}",
    //     *ip_counter, opcode, current_memory_lane, stack
    // );

    let memory_lane = memory_lanes
        .get_mut(*current_memory_lane as usize)
        .ok_or("Invalid memory lane")?;

    match opcode {
        Opcode::Read => {
            let address = pop_i64(stack)? as usize;
            let value = memory_lane.read(address)? as i64;
            stack.push(value as i64);
            *ip_counter += 1;
        }
        Opcode::Write(address) => {
            let value = pop_i64(stack)?;
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
            stack.push(memory_lane.size() as i64);
            *ip_counter += 1;
        }
        Opcode::Noop => {
            *ip_counter += 1;
        }
        Opcode::Pop => {
            let _ = pop_i64(stack)?;
            *ip_counter += 1;
        }
        Opcode::Pop32bits => {
            for _ in 0..4 {
                let _ = pop_i64(stack)?;
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
            let a = pop_i64(stack)?;
            let b = pop_i64(stack)?;
            stack.push(a ^ b);
            *ip_counter += 1;
        }
        Opcode::And => {
            let a = pop_i64(stack)?;
            let b = pop_i64(stack)?;
            stack.push(((a != 0) && (b != 0)) as i64);
            *ip_counter += 1;
        }
        Opcode::Or => {
            let a = pop_i64(stack)?;
            let b = pop_i64(stack)?;
            stack.push(((a != 0) || (b != 0)) as i64);
            *ip_counter += 1;
        }
        Opcode::Zero => {
            stack.push(0);
            *ip_counter += 1;
        }
        Opcode::PushIntermediate(value) => {
            stack.push(*value as i64);
            *ip_counter += 1;
        }
        Opcode::Add => {
            let a = pop_i64(stack)?;
            let b = pop_i64(stack)?;
            stack.push(a.wrapping_add(b));
            *ip_counter += 1;
        }
        Opcode::Sub => {
            let a = pop_i64(stack)?;
            let b = pop_i64(stack)?;
            stack.push(a.wrapping_sub(b));
            *ip_counter += 1;
        }
        Opcode::SmallerThan => {
            let a = pop_i64(stack)?;
            let b = pop_i64(stack)?;
            stack.push((a < b) as i64);
            *ip_counter += 1;
        }
        Opcode::JumpIfTrue => {
            let delta_address = pop_i64(stack)?;
            let condition = pop_i64(stack)?;
            if condition != 0 {
                // TODO add check if we have have a ip counter that is negative
                *ip_counter = ((*ip_counter as i64) + delta_address + 1) as usize;
            } else {
                *ip_counter += 1;
            }
        }
        Opcode::Not => {
            let value = pop_i64(stack)?;
            stack.push((value == 0) as i64);
            *ip_counter += 1;
        }
        Opcode::Print => {
            let value = pop_i64(stack)?;
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
        Opcode::OperationCounter => {
            stack.push(*ip_counter as i64);
            *ip_counter += 1;
        }
        Opcode::Jump => {
            let delta_address = pop_i64(stack)? as i64;
            // TODO add check if we have have a ip counter that is negative
            *ip_counter = ((*ip_counter as i64) + delta_address + 1) as usize;
        }
    }

    Ok(false)
}
