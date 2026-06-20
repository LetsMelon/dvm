use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

pub enum Opcode {
    /// Pops an address from the stack, reads one byte from the current memory lane, and pushes it
    Read,
    /// Pops an address from the stack, then pops a value and writes its low byte there
    Write,
    /// Pops a lane index from the stack and switches to that memory lane
    SwitchMemoryLane,
    SizeOfMemoryLane,
    Noop,
    Pop,
    Pop32bits,
    Dup,
    /// Pops n from the stack and duplicates the top n stack values
    DupN,
    /// Pops n, then direction (0 = left, 1 = right), and rotates the top n stack values once
    Swap,
    Xor,
    /// Pops the top two values and pushes 1 if both are non-zero, otherwise 0
    And,
    /// Pops the top two values and pushes 1 if either is non-zero, otherwise 0
    Or,
    Zero,
    PushIntermediate(i64),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    ShiftLeft,
    ShiftRight,
    SmallerThan,
    SmallerOrEqual,
    Equal,
    GreaterThan,
    GreaterOrEqual,
    /// Pops a delta from the stack and jumps to that opcode if the next stack value is non-zero
    JumpIfTrue,
    Not,
    Print,
    /// Pops n from the stack, prints the top n values as bytes, and removes them
    PrintN,
    /// Pops a delta from the stack and jumps to that opcode unconditionally
    Jump,
    OperationCounter,
    Halt,
}

impl fmt::Debug for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Opcode::{}", self)
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Opcode::Read => write!(f, "Read"),
            Opcode::Write => write!(f, "Write"),
            Opcode::SwitchMemoryLane => write!(f, "SwitchMemoryLane"),
            Opcode::SizeOfMemoryLane => write!(f, "SizeOfMemoryLane"),
            Opcode::Noop => write!(f, "Noop"),
            Opcode::Pop => write!(f, "Pop"),
            Opcode::Pop32bits => write!(f, "Pop32bits"),
            Opcode::Dup => write!(f, "Dup"),
            Opcode::DupN => write!(f, "DupN"),
            Opcode::Swap => write!(f, "Swap"),
            Opcode::Xor => write!(f, "Xor"),
            Opcode::And => write!(f, "And"),
            Opcode::Or => write!(f, "Or"),
            Opcode::Zero => write!(f, "Zero"),
            Opcode::PushIntermediate(value) => write!(f, "PushIntermediate {value}"),
            Opcode::Add => write!(f, "Add"),
            Opcode::Sub => write!(f, "Sub"),
            Opcode::Mul => write!(f, "Mul"),
            Opcode::Div => write!(f, "Div"),
            Opcode::Mod => write!(f, "Mod"),
            Opcode::ShiftLeft => write!(f, "ShiftLeft"),
            Opcode::ShiftRight => write!(f, "ShiftRight"),
            Opcode::SmallerThan => write!(f, "SmallerThan"),
            Opcode::SmallerOrEqual => write!(f, "SmallerOrEqual"),
            Opcode::Equal => write!(f, "Equal"),
            Opcode::GreaterThan => write!(f, "GreaterThan"),
            Opcode::GreaterOrEqual => write!(f, "GreaterOrEqual"),
            Opcode::JumpIfTrue => write!(f, "JumpIfTrue"),
            Opcode::Not => write!(f, "Not"),
            Opcode::Print => write!(f, "Print"),
            Opcode::PrintN => write!(f, "PrintN"),
            Opcode::Jump => write!(f, "Jump"),
            Opcode::OperationCounter => write!(f, "OperationCounter"),
            Opcode::Halt => write!(f, "Halt"),
        }
    }
}

fn parse_i64(arg: Option<&str>, opcode: &str) -> Result<i64, String> {
    let arg = arg.ok_or_else(|| format!("missing argument for {opcode}"))?;
    arg.parse::<i64>()
        .map_err(|e| format!("invalid argument for {opcode}: {e}"))
}

fn ensure_no_extra_args<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    opcode: &str,
) -> Result<(), String> {
    if let Some(extra) = parts.next() {
        Err(format!("unexpected extra argument for {opcode}: {extra}"))
    } else {
        Ok(())
    }
}

impl FromStr for Opcode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        let opcode = parts.next().ok_or_else(|| "empty opcode".to_string())?;

        let parsed = match opcode {
            "Read" => Opcode::Read,
            "Write" => Opcode::Write,
            "SwitchMemoryLane" => Opcode::SwitchMemoryLane,
            "SizeOfMemoryLane" => Opcode::SizeOfMemoryLane,
            "Noop" => Opcode::Noop,
            "Pop" => Opcode::Pop,
            "Pop32bits" => Opcode::Pop32bits,
            "Dup" => Opcode::Dup,
            "DupN" => Opcode::DupN,
            "Swap" => Opcode::Swap,
            "Xor" => Opcode::Xor,
            "And" => Opcode::And,
            "Or" => Opcode::Or,
            "Zero" => Opcode::Zero,
            "PushIntermediate" => Opcode::PushIntermediate(parse_i64(parts.next(), opcode)?),
            "Add" => Opcode::Add,
            "Sub" => Opcode::Sub,
            "Mul" => Opcode::Mul,
            "Div" => Opcode::Div,
            "Mod" => Opcode::Mod,
            "ShiftLeft" => Opcode::ShiftLeft,
            "ShiftRight" => Opcode::ShiftRight,
            "SmallerThan" => Opcode::SmallerThan,
            "SmallerOrEqual" => Opcode::SmallerOrEqual,
            "Equal" => Opcode::Equal,
            "GreaterThan" => Opcode::GreaterThan,
            "GreaterOrEqual" => Opcode::GreaterOrEqual,
            "JumpIfTrue" => Opcode::JumpIfTrue,
            "Not" => Opcode::Not,
            "Print" => Opcode::Print,
            "PrintN" => Opcode::PrintN,
            "Jump" => Opcode::Jump,
            "OperationCounter" => Opcode::OperationCounter,
            "Halt" => Opcode::Halt,
            _ => return Err(format!("unknown opcode: {opcode}")),
        };

        ensure_no_extra_args(&mut parts, opcode)?;
        Ok(parsed)
    }
}

impl Serialize for Opcode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Opcode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}
