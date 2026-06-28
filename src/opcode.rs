use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

#[derive(Clone, PartialEq, Eq)]
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
    I32Xor,
    /// Pops the top two values and pushes 1 if both are non-zero, otherwise 0
    I32And,
    /// Pops the top two values and pushes 1 if either is non-zero, otherwise 0
    I32Or,
    I32Zero,
    I32Push(i32),
    I32Add,
    I32Sub,
    I32Mul,
    I32Div,
    I32Mod,
    I32Shl,
    I32Shr,
    I32Lt,
    I32Le,
    I32Eq,
    I32Gt,
    I32Ge,
    /// Pops a delta from the stack and jumps to that opcode if the next stack value is non-zero
    JumpIfTrue,
    I32Not,
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
            Opcode::I32Xor => write!(f, "i32.XOR"),
            Opcode::I32And => write!(f, "i32.AND"),
            Opcode::I32Or => write!(f, "i32.OR"),
            Opcode::I32Zero => write!(f, "i32.ZERO"),
            Opcode::I32Push(value) => write!(f, "i32.PUSH {value}"),
            Opcode::I32Add => write!(f, "i32.ADD"),
            Opcode::I32Sub => write!(f, "i32.SUB"),
            Opcode::I32Mul => write!(f, "i32.MUL"),
            Opcode::I32Div => write!(f, "i32.DIV"),
            Opcode::I32Mod => write!(f, "i32.MOD"),
            Opcode::I32Shl => write!(f, "i32.SHL"),
            Opcode::I32Shr => write!(f, "i32.SHR"),
            Opcode::I32Lt => write!(f, "i32.LT"),
            Opcode::I32Le => write!(f, "i32.LE"),
            Opcode::I32Eq => write!(f, "i32.EQ"),
            Opcode::I32Gt => write!(f, "i32.GT"),
            Opcode::I32Ge => write!(f, "i32.GE"),
            Opcode::JumpIfTrue => write!(f, "JumpIfTrue"),
            Opcode::I32Not => write!(f, "i32.NOT"),
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

fn parse_i32(arg: Option<&str>, opcode: &str) -> Result<i32, String> {
    let value = parse_i64(arg, opcode)?;
    i32::try_from(value).map_err(|_| format!("argument for {opcode} is out of i32 range: {value}"))
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
            "i32.XOR" => Opcode::I32Xor,
            "i32.AND" => Opcode::I32And,
            "i32.OR" => Opcode::I32Or,
            "i32.ZERO" => Opcode::I32Zero,
            "i32.PUSH" => Opcode::I32Push(parse_i32(parts.next(), opcode)?),
            "i32.ADD" => Opcode::I32Add,
            "i32.SUB" => Opcode::I32Sub,
            "i32.MUL" => Opcode::I32Mul,
            "i32.DIV" => Opcode::I32Div,
            "i32.MOD" => Opcode::I32Mod,
            "i32.SHL" => Opcode::I32Shl,
            "i32.SHR" => Opcode::I32Shr,
            "i32.LT" => Opcode::I32Lt,
            "i32.LE" => Opcode::I32Le,
            "i32.EQ" => Opcode::I32Eq,
            "i32.GT" => Opcode::I32Gt,
            "i32.GE" => Opcode::I32Ge,
            "JumpIfTrue" => Opcode::JumpIfTrue,
            "i32.NOT" => Opcode::I32Not,
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

#[cfg(test)]
mod tests {
    use super::Opcode;

    #[test]
    fn parses_typed_i32_opcodes() {
        assert_eq!("i32.ZERO".parse::<Opcode>().unwrap(), Opcode::I32Zero);
        assert_eq!("i32.ADD".parse::<Opcode>().unwrap(), Opcode::I32Add);
        assert_eq!(
            "i32.PUSH -12".parse::<Opcode>().unwrap(),
            Opcode::I32Push(-12)
        );
    }

    #[test]
    fn rejects_legacy_i32_opcode_spellings() {
        assert!("Zero".parse::<Opcode>().is_err());
        assert!("Add".parse::<Opcode>().is_err());
        assert!("PushIntermediate 3".parse::<Opcode>().is_err());
    }
}
