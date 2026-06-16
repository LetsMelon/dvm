use crate::opcode::Opcode;

pub struct Program<'a> {
    pub(crate) opcodes: &'a [Opcode],
    pub(crate) ip_counter: usize,
    pub(crate) current_memory_lane: u8,
    pub(crate) line_metrics: Vec<u64>,
}

impl<'a> Program<'a> {
    pub fn new(opcodes: &'a [Opcode]) -> Program<'a> {
        Self {
            opcodes,
            ip_counter: 0,
            current_memory_lane: 0,
            line_metrics: vec![0; opcodes.len()],
        }
    }

    pub fn is_outside_program(&self) -> bool {
        self.ip_counter >= self.opcodes.len()
    }

    pub fn get_line_metrics(&self) -> &[u64] {
        &self.line_metrics
    }
}
