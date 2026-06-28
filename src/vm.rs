use std::collections::HashMap;

use crate::{memory_lane::MemoryLane, opcode::Opcode, program::Program, stack::Stack};

type ExternalFunction<'memory> =
    dyn for<'ctx, 'code> FnMut(ExternalFunctionArgs<'ctx, 'code, 'memory>) -> Result<(), String>;

pub struct Vm<'a> {
    op_counter: u64,
    stack: Stack,
    memory_lanes: Box<[MemoryLane<'a>]>,
    external_functions: HashMap<&'static str, Box<ExternalFunction<'a>>>,
}

pub struct ExternalFunctionArgs<'ctx, 'code, 'memory> {
    program: &'ctx Program<'code>,
    stack: &'ctx mut Stack,
    memory_lanes: &'ctx mut [MemoryLane<'memory>],
}

impl<'ctx, 'code, 'memory> ExternalFunctionArgs<'ctx, 'code, 'memory> {
    pub fn program(&self) -> &Program<'code> {
        self.program
    }

    pub fn stack(&mut self) -> &mut Stack {
        self.stack
    }

    pub fn memory_lanes(&mut self) -> &mut [MemoryLane<'memory>] {
        self.memory_lanes
    }

    pub fn instruction_pointer(&self) -> usize {
        self.program.ip_counter
    }

    pub fn current_memory_lane(&self) -> u8 {
        self.program.current_memory_lane
    }

    pub fn current_opcode(&self) -> Option<&Opcode> {
        self.program.get_current_opcode()
    }
}

impl<'a> Vm<'a> {
    pub fn new(memory_lanes: Box<[MemoryLane<'a>]>) -> Self {
        Vm {
            op_counter: 0,
            stack: Stack::new(1024),
            memory_lanes,
            external_functions: HashMap::new(),
        }
    }

    pub fn step<'b>(&mut self, program: &mut Program<'b>) -> Result<Option<i32>, String> {
        self.op_counter += 1;

        step(
            program,
            self.op_counter,
            &mut self.stack,
            &mut self.memory_lanes,
            &mut self.external_functions,
        )
    }

    pub fn execute_opcode<'b>(
        &mut self,
        program: &mut Program<'b>,
        opcode: &Opcode,
    ) -> Result<Option<i32>, String> {
        self.op_counter += 1;
        execute_opcode(
            opcode,
            self.op_counter,
            program,
            &mut self.stack,
            &mut self.memory_lanes,
            &mut self.external_functions,
        )
    }

    pub fn get_op_counter(&self) -> u64 {
        self.op_counter
    }

    pub fn get_stack_bytes(&self) -> Vec<u8> {
        self.stack.dump_bytes()
    }

    pub fn register_external_function<F>(&mut self, name: &'static str, function: F)
    where
        F: for<'ctx, 'code> FnMut(ExternalFunctionArgs<'ctx, 'code, 'a>) -> Result<(), String>
            + 'static,
    {
        self.external_functions.insert(name, Box::new(function));
    }
}

#[inline]
fn step<'memory>(
    program: &mut Program<'_>,
    op_counter: u64,
    stack: &mut Stack,
    memory_lanes: &mut [MemoryLane<'memory>],
    external_functions: &mut HashMap<&'static str, Box<ExternalFunction<'memory>>>,
) -> Result<Option<i32>, String> {
    let opcode = program
        .opcodes
        .get(program.ip_counter)
        .ok_or("Instruction pointer out of bounds")?;

    program.line_metrics[program.ip_counter] += 1;

    execute_opcode(
        opcode,
        op_counter,
        program,
        stack,
        memory_lanes,
        external_functions,
    )
}

#[inline]
fn execute_opcode<'memory>(
    opcode: &Opcode,
    op_counter: u64,
    program: &mut Program<'_>,
    stack: &mut Stack,
    memory_lanes: &mut [MemoryLane<'memory>],
    external_functions: &mut HashMap<&'static str, Box<ExternalFunction<'memory>>>,
) -> Result<Option<i32>, String> {
    match opcode {
        Opcode::Read => {
            let memory_lane = current_memory_lane_mut(memory_lanes, program.current_memory_lane)?;
            let address = i32_to_usize(stack.pop_i32()?, "Read address")?;
            let value = memory_lane.read(address)?;
            stack.push_i32(i32::from(value))?;
            program.ip_counter += 1;
        }
        Opcode::Write => {
            let memory_lane = current_memory_lane_mut(memory_lanes, program.current_memory_lane)?;
            let address = i32_to_usize(stack.pop_i32()?, "Write address")?;
            let value = stack.pop_i32()?;
            if let MemoryLane::ReadWrite(slice) = memory_lane {
                slice[address] = (value & 0xFF) as u8;
            } else {
                return Err("Cannot write to read-only memory lane".into());
            }
            program.ip_counter += 1;
        }
        Opcode::SwitchMemoryLane => {
            program.current_memory_lane = i32_to_u8(stack.pop_i32()?, "Memory lane index")?;
            program.ip_counter += 1;
        }
        Opcode::SizeOfMemoryLane => {
            let memory_lane = current_memory_lane_mut(memory_lanes, program.current_memory_lane)?;
            let size = i32::try_from(memory_lane.size())
                .map_err(|_| "Memory lane size exceeds i32 range".to_string())?;
            stack.push_i32(size)?;
            program.ip_counter += 1;
        }
        Opcode::Noop => {
            program.ip_counter += 1;
        }
        Opcode::Pop => {
            let _ = stack.pop_i32()?;
            program.ip_counter += 1;
        }
        Opcode::Pop32bits => {
            let mut bytes = [0; 4];
            stack.pop_bytes(&mut bytes)?;
            program.ip_counter += 1;
        }
        Opcode::Dup => {
            let value = stack.peek_i32()?;
            stack.push_i32(value)?;
            program.ip_counter += 1;
        }
        Opcode::DupN => {
            let n = i32_to_usize(stack.pop_i32()?, "DupN count")?;
            if n == 0 {
                return Err("DupN requires n >= 1".into());
            }

            let len = stack.len_i32s()?;
            if n > len {
                return Err("Stack underflow".into());
            }

            for i in 0..n {
                let value = stack.get_i32(len - n + i)?;
                stack.push_i32(value)?;
            }

            program.ip_counter += 1;
        }
        Opcode::Swap => {
            let n = i32_to_usize(stack.pop_i32()?, "Swap count")?;
            if n < 2 {
                return Err("Swap requires n >= 2".into());
            }

            let direction = stack.pop_i32()?;
            let len = stack.len_i32s()?;
            if n > len {
                return Err("Stack underflow".into());
            }

            match direction {
                0 => stack.rotate_left_once_last_n_i32(n)?,
                1 => {
                    for _ in 0..(n - 1) {
                        stack.rotate_left_once_last_n_i32(n)?;
                    }
                }
                _ => return Err("Swap direction must be 0 (left) or 1 (right)".into()),
            }

            program.ip_counter += 1;
        }
        Opcode::I32Xor => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(a ^ b)?;
            program.ip_counter += 1;
        }
        Opcode::I32And => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(((a != 0) && (b != 0)) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::I32Or => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(((a != 0) || (b != 0)) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::I32Zero => {
            stack.push_i32(0)?;
            program.ip_counter += 1;
        }
        Opcode::I32Push(value) => {
            stack.push_i32(*value)?;
            program.ip_counter += 1;
        }
        Opcode::I32Add => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(a.wrapping_add(b))?;
            program.ip_counter += 1;
        }
        Opcode::I32Sub => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(b.wrapping_sub(a))?;
            program.ip_counter += 1;
        }
        Opcode::I32Mul => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(a.wrapping_mul(b))?;
            program.ip_counter += 1;
        }
        Opcode::I32Div => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(b.wrapping_div(a))?;
            program.ip_counter += 1;
        }
        Opcode::I32Mod => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(b % a)?;
            program.ip_counter += 1;
        }
        Opcode::I32Shl => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(b << a)?;
            program.ip_counter += 1;
        }
        Opcode::I32Shr => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32(b >> a)?;
            program.ip_counter += 1;
        }
        Opcode::I32Lt => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32((b < a) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::I32Le => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32((b <= a) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::I32Eq => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32((a == b) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::I32Gt => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32((b > a) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::I32Ge => {
            let a = stack.pop_i32()?;
            let b = stack.pop_i32()?;
            stack.push_i32((b >= a) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::JumpIfTrue => {
            let delta_address = stack.pop_i32()?;
            let condition = stack.pop_i32()?;
            if condition != 0 {
                advance_ip_relative(&mut program.ip_counter, delta_address)?;
            } else {
                program.ip_counter += 1;
            }
        }
        Opcode::I32Not => {
            let value = stack.pop_i32()?;
            stack.push_i32((value == 0) as i32)?;
            program.ip_counter += 1;
        }
        Opcode::Print => {
            let value = stack.pop_i32()?;
            print!("{}", value as u8 as char);
            program.ip_counter += 1;
        }
        Opcode::PrintN => {
            let size = i32_to_usize(stack.pop_i32()?, "PrintN size")?;
            let len = stack.len_i32s()?;
            if size > len {
                return Err("Stack underflow".into());
            }

            for i in 0..size {
                let value = stack.get_i32(len - size + i)?;
                print!("{}", value as u8 as char);
            }

            for _ in 0..size {
                stack.pop_i32()?;
            }

            program.ip_counter += 1;
        }
        Opcode::OperationCounter => {
            let counter = i32::try_from(op_counter)
                .map_err(|_| "Operation counter exceeds i32 range".to_string())?;
            stack.push_i32(counter)?;
            program.ip_counter += 1;
        }
        Opcode::Jump => {
            let delta_address = stack.pop_i32()?;
            advance_ip_relative(&mut program.ip_counter, delta_address)?;
        }
        Opcode::Halt => {
            let exit_code = stack.pop_i32()?;
            return Ok(Some(exit_code));
        }
        Opcode::CallExternal(function_name) => {
            let fct = external_functions.get_mut(function_name).ok_or_else(|| {
                format!(
                    "Could not get the external function by name: '{}'",
                    function_name
                )
            })?;

            fct(ExternalFunctionArgs {
                program: &*program,
                stack,
                memory_lanes,
            })?;

            program.ip_counter += 1;
        }
    }

    Ok(None)
}

fn i32_to_usize(value: i32, context: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("{context} must be non-negative"))
}

fn i32_to_u8(value: i32, context: &str) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| format!("{context} must fit into u8"))
}

fn current_memory_lane_mut<'ctx, 'memory>(
    memory_lanes: &'ctx mut [MemoryLane<'memory>],
    current_memory_lane: u8,
) -> Result<&'ctx mut MemoryLane<'memory>, String> {
    memory_lanes
        .get_mut(usize::from(current_memory_lane))
        .ok_or("Invalid memory lane".to_string())
}

fn advance_ip_relative(ip_counter: &mut usize, delta: i32) -> Result<(), String> {
    let jump = isize::try_from(delta)
        .map_err(|_| "Jump delta does not fit into isize".to_string())?
        .checked_add(1)
        .ok_or("Jump delta overflow".to_string())?;

    *ip_counter = ip_counter
        .checked_add_signed(jump)
        .ok_or("Instruction pointer out of bounds".to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{frontend::compile_source, memory_lane::MemoryLane};

    use super::*;

    fn run_program(source: &str) -> i32 {
        let mut heap_memory_lane = [0; 128];
        let io_memory_lane = *b"";
        let memory_lanes = [
            MemoryLane::ReadWrite(&mut heap_memory_lane),
            MemoryLane::ReadOnly(&io_memory_lane),
        ];

        let opcodes = compile_source("<test>", source).unwrap();
        let mut program = Program::new(&opcodes);
        let mut vm = Vm::new(Box::new(memory_lanes));

        loop {
            if let Some(code) = vm.step(&mut program).unwrap() {
                return code;
            }

            if program.is_outside_program() {
                panic!("program terminated without Halt");
            }
        }
    }

    #[test]
    fn executes_typed_i32_arithmetic_program() {
        let exit_code = run_program(
            "\
i32.PUSH 3
i32.PUSH 4
i32.ADD
Halt
",
        );

        assert_eq!(exit_code, 7);
    }

    #[test]
    fn executes_symbolic_jump_program_with_typed_i32_push() {
        let exit_code = run_program(
            "\
i32.PUSH 1
JumpIfTrue .done
i32.PUSH 99
.done
i32.PUSH 7
Halt
",
        );

        assert_eq!(exit_code, 7);
    }

    #[test]
    fn calls_registered_external_function() {
        let mut heap_memory_lane = [0; 128];
        let io_memory_lane = *b"";
        let memory_lanes = [
            MemoryLane::ReadWrite(&mut heap_memory_lane),
            MemoryLane::ReadOnly(&io_memory_lane),
        ];

        let opcodes = compile_source(
            "<test>",
            "\
i32.PUSH 41
CallExternal add_one
Halt
",
        )
        .unwrap();
        let mut program = Program::new(&opcodes);
        let mut vm = Vm::new(Box::new(memory_lanes));
        vm.register_external_function("add_one", |mut args| {
            let value = args.stack().pop_i32()?;
            args.stack().push_i32(value + 1)?;
            Ok(())
        });

        let exit_code = loop {
            if let Some(code) = vm.step(&mut program).unwrap() {
                break code;
            }

            if program.is_outside_program() {
                panic!("program terminated without Halt");
            }
        };

        assert_eq!(exit_code, 42);
    }
}
