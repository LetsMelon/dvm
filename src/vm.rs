use crate::{
    memory_lane::MemoryLane,
    opcode::Opcode,
    program::Program,
    stack::Stack,
};

pub struct Vm<'a> {
    op_counter: u64,
    stack: Stack,
    memory_lanes: Box<[MemoryLane<'a>]>,
}

impl<'a> Vm<'a> {
    pub fn new(memory_lanes: Box<[MemoryLane<'a>]>) -> Self {
        Vm {
            op_counter: 0,
            stack: Stack::new(128),
            memory_lanes,
        }
    }

    pub fn step<'b>(&mut self, program: &mut Program<'b>) -> Result<bool, String> {
        self.op_counter += 1;

        step(program, &mut self.stack, &mut self.memory_lanes)
    }

    pub fn execute_opcode<'b>(
        &mut self,
        program: &mut Program<'b>,
        opcode: &Opcode,
    ) -> Result<bool, String> {
        self.op_counter += 1;
        execute_opcode(
            opcode,
            &mut program.ip_counter,
            &mut program.current_memory_lane,
            &mut self.stack,
            &mut self.memory_lanes,
        )
    }

    pub fn get_op_counter(&self) -> u64 {
        self.op_counter
    }

    pub fn get_stack(&self) -> &[i64] {
        self.stack.as_slice()
    }
}

#[inline]
fn step(
    program: &mut Program<'_>,
    stack: &mut Stack,
    memory_lanes: &mut [MemoryLane],
) -> Result<bool, String> {
    let opcode = program
        .opcodes
        .get(program.ip_counter)
        .ok_or("Instruction pointer out of bounds")?;

    program.line_metrics[program.ip_counter] += 1;

    execute_opcode(
        opcode,
        &mut program.ip_counter,
        &mut program.current_memory_lane,
        stack,
        memory_lanes,
    )
}

#[inline]
fn execute_opcode(
    opcode: &Opcode,
    ip_counter: &mut usize,
    current_memory_lane: &mut u8,
    stack: &mut Stack,
    memory_lanes: &mut [MemoryLane],
) -> Result<bool, String> {
    let memory_lane = memory_lanes
        .get_mut(*current_memory_lane as usize)
        .ok_or("Invalid memory lane")?;

    match opcode {
        Opcode::Read => {
            let address = stack.pop()? as usize;
            let value = memory_lane.read(address)? as i64;
            stack.push(value as i64);
            *ip_counter += 1;
        }
        Opcode::Write => {
            let address = stack.pop()? as usize;
            let value = stack.pop()?;
            if let MemoryLane::ReadWrite(slice) = memory_lane {
                let value_byte = (value & 0xFF) as u8;
                slice[address] = value_byte;
            } else {
                return Err("Cannot write to read-only memory lane".into());
            }
            *ip_counter += 1;
        }
        Opcode::SwitchMemoryLane => {
            *current_memory_lane = stack.pop()? as u8;
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
            let _ = stack.pop()?;
            *ip_counter += 1;
        }
        Opcode::Pop32bits => {
            for _ in 0..4 {
                let _ = stack.pop()?;
            }
            *ip_counter += 1;
        }
        Opcode::Dup => {
            let value = stack.top()?;
            stack.push(value);
            *ip_counter += 1;
        }
        Opcode::DupN => {
            let n = stack.pop()? as usize;
            if n == 0 {
                return Err("DupN requires n >= 1".into());
            }

            let len = stack.size();
            if n > len {
                return Err("Stack underflow".into());
            }

            for i in 0..n {
                let value = stack.get(len - n + i)?;
                stack.push(value);
            }

            *ip_counter += 1;
        }
        Opcode::Swap => {
            let n = stack.pop()? as usize;
            if n < 2 {
                return Err("Swap requires n >= 2".into());
            }

            let direction = stack.pop()?;
            let len = stack.size();
            if n > len {
                return Err("Stack underflow".into());
            }

            match direction {
                0 => stack.rotate_left_once_last_n(n)?,
                1 => {
                    for _ in 0..(n - 1) {
                        stack.rotate_left_once_last_n(n)?;
                    }
                }
                _ => return Err("Swap direction must be 0 (left) or 1 (right)".into()),
            }

            *ip_counter += 1;
        }
        Opcode::Xor => {
            let a = stack.pop()?;
            let b = stack.pop()?;
            stack.push(a ^ b);
            *ip_counter += 1;
        }
        Opcode::And => {
            let a = stack.pop()?;
            let b = stack.pop()?;
            stack.push(((a != 0) && (b != 0)) as i64);
            *ip_counter += 1;
        }
        Opcode::Or => {
            let a = stack.pop()?;
            let b = stack.pop()?;
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
            let a = stack.pop()?;
            let b = stack.pop()?;
            stack.push(a.wrapping_add(b));
            *ip_counter += 1;
        }
        Opcode::Sub => {
            let a = stack.pop()?;
            let b = stack.pop()?;
            stack.push(a.wrapping_sub(b));
            *ip_counter += 1;
        }
        Opcode::SmallerThan => {
            let a = stack.pop()?;
            let b = stack.pop()?;
            stack.push((a < b) as i64);
            *ip_counter += 1;
        }
        Opcode::JumpIfTrue => {
            let delta_address = stack.pop()?;
            let condition = stack.pop()?;
            if condition != 0 {
                // TODO add check if we have have a ip counter that is negative
                *ip_counter = ((*ip_counter as i64) + delta_address + 1) as usize;
            } else {
                *ip_counter += 1;
            }
        }
        Opcode::Not => {
            let value = stack.pop()?;
            stack.push((value == 0) as i64);
            *ip_counter += 1;
        }
        Opcode::Print => {
            let value = stack.pop()?;
            print!("{}", value as u8 as char);
            *ip_counter += 1;
        }
        Opcode::PrintN => {
            let size = stack.pop()? as usize;
            let len = stack.size();
            if size > len {
                return Err("Stack underflow".into());
            }

            for i in 0..size {
                let value = stack.get(len - size + i)?;
                print!("{}", value as u8 as char);
            }

            for _ in 0..size {
                stack.pop()?;
            }

            *ip_counter += 1;
        }
        Opcode::OperationCounter => {
            stack.push(*ip_counter as i64);
            *ip_counter += 1;
        }
        Opcode::Jump => {
            let delta_address = stack.pop()? as i64;
            // TODO add check if we have have a ip counter that is negative
            *ip_counter = ((*ip_counter as i64) + delta_address + 1) as usize;
        }
    }

    Ok(false)
}
