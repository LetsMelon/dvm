#[derive(Debug)]
pub enum Opcode {
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
    /// Pops the top two values and pushes 1 if both are non-zero, otherwise 0
    And,
    /// Pops the top two values and pushes 1 if either is non-zero, otherwise 0
    Or,
    Zero,
    PushIntermediate(i64),
    Add,
    Sub,
    SmallerThan,
    /// Pops a delta from the stack and jumps to that opcode if the next stack value is non-zero
    JumpIfTrue,
    Not,
    Print,
    PrintN(u8),
    /// Pops a delta from the stack and jumps to that opcode unconditionally
    Jump,
    OperationCounter,
}

#[derive(Debug, Clone, Copy)]
pub enum SwapDirection {
    Left,
    Right,
}
