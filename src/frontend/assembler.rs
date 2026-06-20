use std::collections::HashMap;

use crate::opcode::Opcode;

use super::ir::{IrInstructionKind, IrProgram};

pub(crate) fn assemble_ir(path: &str, ir: &IrProgram) -> Result<Vec<Opcode>, String> {
    if ir.instructions.is_empty() {
        return Err(format!("program file {path} did not contain any opcodes"));
    }

    let mut label_positions = HashMap::<String, (usize, usize)>::new();
    let mut emitted_positions = Vec::with_capacity(ir.instructions.len());
    let mut emitted_count = 0usize;

    for instruction in &ir.instructions {
        emitted_positions.push(emitted_count);

        for label in &instruction.labels_here {
            if let Some((_, previous_line)) =
                label_positions.insert(label.name.clone(), (emitted_count, label.source_line))
            {
                return Err(format!(
                    "{path}:{}: duplicate label {} (previously defined on line {})",
                    label.source_line, label.name, previous_line
                ));
            }
        }

        emitted_count += emitted_width(&instruction.kind);
    }

    let mut opcodes = Vec::with_capacity(emitted_count);

    for (index, instruction) in ir.instructions.iter().enumerate() {
        match &instruction.kind {
            IrInstructionKind::Concrete(opcode) => opcodes.push(opcode.clone()),
            IrInstructionKind::JumpLabel(label) => {
                let (target_position, _) = label_positions.get(label).ok_or_else(|| {
                    format!(
                        "{path}:{}: unknown label {}",
                        instruction.source_line, label
                    )
                })?;
                let jump_position = emitted_positions[index] + 1;
                let delta = *target_position as i64 - jump_position as i64 - 1;
                opcodes.push(Opcode::PushIntermediate(delta));
                opcodes.push(Opcode::Jump);
            }
            IrInstructionKind::JumpIfTrueLabel(label) => {
                let (target_position, _) = label_positions.get(label).ok_or_else(|| {
                    format!(
                        "{path}:{}: unknown label {}",
                        instruction.source_line, label
                    )
                })?;
                let jump_position = emitted_positions[index] + 1;
                let delta = *target_position as i64 - jump_position as i64 - 1;
                opcodes.push(Opcode::PushIntermediate(delta));
                opcodes.push(Opcode::JumpIfTrue);
            }
        }
    }

    Ok(opcodes)
}

fn emitted_width(kind: &IrInstructionKind) -> usize {
    match kind {
        IrInstructionKind::Concrete(_) => 1,
        IrInstructionKind::JumpLabel(_) | IrInstructionKind::JumpIfTrueLabel(_) => 2,
    }
}
