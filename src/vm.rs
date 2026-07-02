use std::collections::HashMap;

use crate::{memory_lane::MemoryLane, opcode::Opcode, program::Program, stack::Stack};

type ExternalFunction<'memory> =
    dyn for<'ctx, 'code> FnMut(ExternalFunctionArgs<'ctx, 'code, 'memory>) -> Result<(), String>;

macro_rules! opcode_handler {
    (fn $name:ident $(<$($generic:tt),*>)? ($($arg:ident : $ty:ty),* $(,)?) -> $ret:ty $body:block) => {
        #[cfg_attr(feature = "profiling", inline(never))]
        #[cfg_attr(not(feature = "profiling"), inline(always))]
        fn $name $(<$($generic),*>)? ($($arg: $ty),*) -> $ret $body
    };
}

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
        Opcode::Read => execute_read(program, stack, memory_lanes),
        Opcode::Write => execute_write(program, stack, memory_lanes),
        Opcode::SwitchMemoryLane => execute_switch_memory_lane(program, stack),
        Opcode::SizeOfMemoryLane => execute_size_of_memory_lane(program, stack, memory_lanes),
        Opcode::Noop => execute_noop(program),
        Opcode::Pop => execute_pop(program, stack),
        Opcode::Pop32bits => execute_pop32bits(program, stack),
        Opcode::Dup => execute_dup(program, stack),
        Opcode::DupN => execute_dup_n(program, stack),
        Opcode::Swap => execute_swap(program, stack),
        Opcode::I32Xor => execute_i32_xor(program, stack),
        Opcode::I32And => execute_i32_and(program, stack),
        Opcode::I32Or => execute_i32_or(program, stack),
        Opcode::I32Zero => execute_i32_zero(program, stack),
        Opcode::I32Push(value) => execute_i32_push(program, stack, *value),
        Opcode::I32PickN => execute_i32_pick_n(program, stack),
        Opcode::I32Add => execute_i32_add(program, stack),
        Opcode::I32Sub => execute_i32_sub(program, stack),
        Opcode::I32Mul => execute_i32_mul(program, stack),
        Opcode::I32Div => execute_i32_div(program, stack),
        Opcode::I32Mod => execute_i32_mod(program, stack),
        Opcode::I32Shl => execute_i32_shl(program, stack),
        Opcode::I32Shr => execute_i32_shr(program, stack),
        Opcode::I32Lt => execute_i32_lt(program, stack),
        Opcode::I32Le => execute_i32_le(program, stack),
        Opcode::I32Eq => execute_i32_eq(program, stack),
        Opcode::I32Gt => execute_i32_gt(program, stack),
        Opcode::I32Ge => execute_i32_ge(program, stack),
        Opcode::I32Not => execute_i32_not(program, stack),
        Opcode::I32ToF32 => execute_i32_to_f32(program, stack),
        Opcode::F32Push(value) => execute_f32_push(program, stack, *value),
        Opcode::F32Add => execute_f32_add(program, stack),
        Opcode::F32Sub => execute_f32_sub(program, stack),
        Opcode::F32Mul => execute_f32_mul(program, stack),
        Opcode::F32Div => execute_f32_div(program, stack),
        Opcode::F32Eq => execute_i32_eq(program, stack),
        Opcode::F32Gt => execute_f32_gt(program, stack),
        Opcode::F32Ge => execute_f32_ge(program, stack),
        Opcode::F32ToI32 => execute_f32_to_i32(program, stack),
        Opcode::JumpIfTrue => execute_jump_if_true(program, stack),
        Opcode::Print => execute_print(program, stack),
        Opcode::PrintN => execute_print_n(program, stack),
        Opcode::Jump => execute_jump(program, stack),
        Opcode::OperationCounter => execute_operation_counter(program, stack, op_counter),
        Opcode::Halt => execute_halt(stack),
        Opcode::CallExternal(function_name) => execute_call_external(
            program,
            stack,
            memory_lanes,
            external_functions,
            function_name,
        ),
    }
}

opcode_handler! {
    fn execute_read<'memory>(
        program: &mut Program<'_>,
        stack: &mut Stack,
        memory_lanes: &mut [MemoryLane<'memory>],
    ) -> Result<Option<i32>, String> {
        let memory_lane = current_memory_lane_mut(memory_lanes, program.current_memory_lane)?;
        let address = i32_to_usize(stack.pop_i32()?, "Read address")?;
        let value = memory_lane.read(address)?;
        stack.push_i32(i32::from(value))?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_write<'memory>(
        program: &mut Program<'_>,
        stack: &mut Stack,
        memory_lanes: &mut [MemoryLane<'memory>],
    ) -> Result<Option<i32>, String> {
        let memory_lane = current_memory_lane_mut(memory_lanes, program.current_memory_lane)?;
        let address = i32_to_usize(stack.pop_i32()?, "Write address")?;
        let value = stack.pop_i32()?;
        if let MemoryLane::ReadWrite(slice) = memory_lane {
            slice[address] = (value & 0xFF) as u8;
        } else {
            return Err("Cannot write to read-only memory lane".into());
        }
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_switch_memory_lane(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        program.current_memory_lane = i32_to_u8(stack.pop_i32()?, "Memory lane index")?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_size_of_memory_lane<'memory>(
        program: &mut Program<'_>,
        stack: &mut Stack,
        memory_lanes: &mut [MemoryLane<'memory>],
    ) -> Result<Option<i32>, String> {
        let memory_lane = current_memory_lane_mut(memory_lanes, program.current_memory_lane)?;
        let size = i32::try_from(memory_lane.size())
            .map_err(|_| "Memory lane size exceeds i32 range".to_string())?;
        stack.push_i32(size)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_noop(program: &mut Program<'_>) -> Result<Option<i32>, String> {
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_pop(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let _ = stack.pop_i32()?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_pop32bits(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let mut bytes = [0; 4];
        stack.pop_bytes(&mut bytes)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_dup(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let value = stack.peek_i32()?;
        stack.push_i32(value)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_dup_n(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
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
        Ok(None)
    }
}

opcode_handler! {
    fn execute_swap(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
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
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_xor(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(a ^ b)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_and(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(((a != 0) && (b != 0)) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_or(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(((a != 0) || (b != 0)) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_zero(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        stack.push_i32(0)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_push(
        program: &mut Program<'_>,
        stack: &mut Stack,
        value: i32,
    ) -> Result<Option<i32>, String> {
        stack.push_i32(value)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_pick_n(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let n = stack.pop_i32()?;
        if !(1..=8).contains(&n) {
            return Err("i32.PickN requires n between 1 and 8".into());
        }

        let n = usize::try_from(n).expect("n is validated to be positive");
        let len = stack.len_i32s()?;
        if n > len {
            return Err("Stack underflow".into());
        }

        let value = stack.get_i32(len - n)?;
        stack.push_i32(value)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_add(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(a.wrapping_add(b))?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_sub(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(b.wrapping_sub(a))?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_mul(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(a.wrapping_mul(b))?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_div(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(b.wrapping_div(a))?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_mod(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(b % a)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_shl(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(b << a)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_shr(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32(b >> a)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_lt(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32((b < a) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_le(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32((b <= a) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_eq(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32((a == b) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_gt(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32((b > a) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_ge(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_i32()?;
        let b = stack.pop_i32()?;
        stack.push_i32((b >= a) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_jump_if_true(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let delta_address = stack.pop_i32()?;
        let condition = stack.pop_i32()?;
        if condition != 0 {
            advance_ip_relative(&mut program.ip_counter, delta_address)?;
        } else {
            program.ip_counter += 1;
        }
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_not(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let value = stack.pop_i32()?;
        stack.push_i32((value == 0) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_i32_to_f32(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let value = stack.pop_i32()?;
        stack.push_f32(value as f32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_print(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let value = stack.pop_i32()?;
        print!("{}", value as u8 as char);
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_push(
        program: &mut Program<'_>,
        stack: &mut Stack,
        value: f32,
    ) -> Result<Option<i32>, String> {
        stack.push_f32(value)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_add(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_f32()?;
        let b = stack.pop_f32()?;
        stack.push_f32(a + b)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_sub(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_f32()?;
        let b = stack.pop_f32()?;
        stack.push_f32(b - a)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_mul(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_f32()?;
        let b = stack.pop_f32()?;
        stack.push_f32(a * b)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_div(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_f32()?;
        let b = stack.pop_f32()?;
        stack.push_f32(b / a)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_gt(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_f32()?;
        let b = stack.pop_f32()?;
        stack.push_i32((b > a) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_ge(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let a = stack.pop_f32()?;
        let b = stack.pop_f32()?;
        stack.push_i32((b >= a) as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_f32_to_i32(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let value = stack.pop_f32()?;
        stack.push_i32(value.round() as i32)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_print_n(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
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
        Ok(None)
    }
}

opcode_handler! {
    fn execute_operation_counter(
        program: &mut Program<'_>,
        stack: &mut Stack,
        op_counter: u64,
    ) -> Result<Option<i32>, String> {
        let counter = i32::try_from(op_counter)
            .map_err(|_| "Operation counter exceeds i32 range".to_string())?;
        stack.push_i32(counter)?;
        program.ip_counter += 1;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_jump(
        program: &mut Program<'_>,
        stack: &mut Stack,
    ) -> Result<Option<i32>, String> {
        let delta_address = stack.pop_i32()?;
        advance_ip_relative(&mut program.ip_counter, delta_address)?;
        Ok(None)
    }
}

opcode_handler! {
    fn execute_halt(stack: &mut Stack) -> Result<Option<i32>, String> {
        let exit_code = stack.pop_i32()?;
        Ok(Some(exit_code))
    }
}

opcode_handler! {
    fn execute_call_external<'memory>(
        program: &mut Program<'_>,
        stack: &mut Stack,
        memory_lanes: &mut [MemoryLane<'memory>],
        external_functions: &mut HashMap<&'static str, Box<ExternalFunction<'memory>>>,
        function_name: &'static str,
    ) -> Result<Option<i32>, String> {
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
        Ok(None)
    }
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

    fn run_program_error(source: &str) -> String {
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
            match vm.step(&mut program) {
                Ok(Some(_)) => panic!("program terminated without error"),
                Ok(None) => {}
                Err(error) => return error,
            }

            if program.is_outside_program() {
                panic!("program terminated without error");
            }
        }
    }

    fn run_f32_binary_program(opcode: &str, lhs: f32, rhs: f32, expected: f32) {
        let mut heap_memory_lane = [0; 128];
        let io_memory_lane = *b"";
        let memory_lanes = [
            MemoryLane::ReadWrite(&mut heap_memory_lane),
            MemoryLane::ReadOnly(&io_memory_lane),
        ];

        let source = format!(
            "\
f32.PUSH {lhs}
f32.PUSH {rhs}
{opcode}
CallExternal assert_result
i32.PUSH 0
Halt
"
        );
        let opcodes = compile_source("<test>", &source).unwrap();
        let mut program = Program::new(&opcodes);
        let mut vm = Vm::new(Box::new(memory_lanes));

        vm.register_external_function("assert_result", move |mut args| {
            let actual = args.stack().pop_f32()?;
            if (actual - expected).abs() > f32::EPSILON {
                return Err(format!("expected {expected}, got {actual}"));
            }
            Ok(())
        });

        loop {
            if let Some(code) = vm.step(&mut program).unwrap() {
                assert_eq!(code, 0);
                return;
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
    fn executes_i32_pick_n() {
        let exit_code = run_program(
            "\
i32.PUSH 11
i32.PUSH 22
i32.PUSH 2
i32.PickN
Halt
",
        );

        assert_eq!(exit_code, 11);
    }

    #[test]
    fn i32_pick_n_copies_f32_bits() {
        let exit_code = run_program(
            "\
f32.PUSH 1.5
i32.PUSH 1
i32.PickN
f32.TO.i32
Halt
",
        );

        assert_eq!(exit_code, 2);
    }

    #[test]
    fn i32_pick_n_validates_range() {
        let error = run_program_error(
            "\
i32.PUSH 11
i32.PUSH 9
i32.PickN
Halt
",
        );

        assert!(error.contains("i32.PickN requires n between 1 and 8"));
    }

    #[test]
    fn executes_typed_f32_arithmetic_programs() {
        run_f32_binary_program("f32.SUB", 10.5, 4.25, 6.25);
        run_f32_binary_program("f32.MUL", 2.5, 4.0, 10.0);
        run_f32_binary_program("f32.DIV", 9.0, 3.0, 3.0);
    }

    #[test]
    fn executes_typed_f32_comparison_programs() {
        assert_eq!(
            run_program(
                "\
f32.PUSH 4.5
f32.PUSH 4.5
f32.EQ
Halt
"
            ),
            1
        );
        assert_eq!(
            run_program(
                "\
f32.PUSH 4.5
f32.PUSH 3.5
f32.GT
Halt
"
            ),
            1
        );
        assert_eq!(
            run_program(
                "\
f32.PUSH 4.5
f32.PUSH 4.5
f32.GE
Halt
"
            ),
            1
        );
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
