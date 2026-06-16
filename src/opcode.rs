use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

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

impl fmt::Debug for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Opcode::{}", self)
    }
}

impl fmt::Display for SwapDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapDirection::Left => write!(f, "Left"),
            SwapDirection::Right => write!(f, "Right"),
        }
    }
}

impl FromStr for SwapDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Left" => Ok(SwapDirection::Left),
            "Right" => Ok(SwapDirection::Right),
            _ => Err(format!("invalid swap direction: {s}")),
        }
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Opcode::Read => write!(f, "Read"),
            Opcode::Write(address) => write!(f, "Write {address}"),
            Opcode::SwitchMemoryLane(lane) => write!(f, "SwitchMemoryLane {lane}"),
            Opcode::SizeOfMemoryLane => write!(f, "SizeOfMemoryLane"),
            Opcode::Noop => write!(f, "Noop"),
            Opcode::Pop => write!(f, "Pop"),
            Opcode::Pop32bits => write!(f, "Pop32bits"),
            Opcode::Dup => write!(f, "Dup"),
            Opcode::DupN(n) => write!(f, "DupN {n}"),
            Opcode::Swap(direction, n) => write!(f, "Swap {direction} {n}"),
            Opcode::Xor => write!(f, "Xor"),
            Opcode::And => write!(f, "And"),
            Opcode::Or => write!(f, "Or"),
            Opcode::Zero => write!(f, "Zero"),
            Opcode::PushIntermediate(value) => write!(f, "PushIntermediate {value}"),
            Opcode::Add => write!(f, "Add"),
            Opcode::Sub => write!(f, "Sub"),
            Opcode::SmallerThan => write!(f, "SmallerThan"),
            Opcode::JumpIfTrue => write!(f, "JumpIfTrue"),
            Opcode::Not => write!(f, "Not"),
            Opcode::Print => write!(f, "Print"),
            Opcode::PrintN(size) => write!(f, "PrintN {size}"),
            Opcode::Jump => write!(f, "Jump"),
            Opcode::OperationCounter => write!(f, "OperationCounter"),
        }
    }
}

fn parse_u8(arg: Option<&str>, opcode: &str) -> Result<u8, String> {
    let arg = arg.ok_or_else(|| format!("missing argument for {opcode}"))?;
    arg.parse::<u8>()
        .map_err(|e| format!("invalid argument for {opcode}: {e}"))
}

fn parse_u64(arg: Option<&str>, opcode: &str) -> Result<u64, String> {
    let arg = arg.ok_or_else(|| format!("missing argument for {opcode}"))?;
    arg.parse::<u64>()
        .map_err(|e| format!("invalid argument for {opcode}: {e}"))
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
            "Write" => Opcode::Write(parse_u64(parts.next(), opcode)?),
            "SwitchMemoryLane" => Opcode::SwitchMemoryLane(parse_u8(parts.next(), opcode)?),
            "SizeOfMemoryLane" => Opcode::SizeOfMemoryLane,
            "Noop" => Opcode::Noop,
            "Pop" => Opcode::Pop,
            "Pop32bits" => Opcode::Pop32bits,
            "Dup" => Opcode::Dup,
            "DupN" => Opcode::DupN(parse_u8(parts.next(), opcode)?),
            "Swap" => {
                let direction = parts
                    .next()
                    .ok_or_else(|| format!("missing direction for {opcode}"))?
                    .parse::<SwapDirection>()?;
                let n = parse_u8(parts.next(), opcode)?;
                Opcode::Swap(direction, n)
            }
            "Xor" => Opcode::Xor,
            "And" => Opcode::And,
            "Or" => Opcode::Or,
            "Zero" => Opcode::Zero,
            "PushIntermediate" => Opcode::PushIntermediate(parse_i64(parts.next(), opcode)?),
            "Add" => Opcode::Add,
            "Sub" => Opcode::Sub,
            "SmallerThan" => Opcode::SmallerThan,
            "JumpIfTrue" => Opcode::JumpIfTrue,
            "Not" => Opcode::Not,
            "Print" => Opcode::Print,
            "PrintN" => Opcode::PrintN(parse_u8(parts.next(), opcode)?),
            "Jump" => Opcode::Jump,
            "OperationCounter" => Opcode::OperationCounter,
            _ => return Err(format!("unknown opcode: {opcode}")),
        };

        ensure_no_extra_args(&mut parts, opcode)?;
        Ok(parsed)
    }
}

impl Serialize for SwapDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SwapDirection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
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
