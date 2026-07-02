use std::collections::HashMap;

use crate::opcode::Opcode;

use super::ir::{IrInstructionKind, IrProgram};

pub struct AssembledProgram {
    pub opcodes: Vec<Opcode>,
    pub metadata: ProgramMetadata,
}

pub struct ProgramMetadata {
    pub labels_by_ip: Vec<Vec<String>>,
    pub source_lines_by_ip: Vec<usize>,
}

#[cfg(test)]
pub(crate) fn assemble_ir(path: &str, ir: &IrProgram) -> Result<Vec<Opcode>, String> {
    Ok(assemble_ir_with_metadata(path, ir)?.opcodes)
}

pub(crate) fn assemble_ir_with_metadata(
    path: &str,
    ir: &IrProgram,
) -> Result<AssembledProgram, String> {
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
    let mut labels_by_ip = vec![Vec::new(); emitted_count];
    let mut source_lines_by_ip = Vec::with_capacity(emitted_count);

    for (index, instruction) in ir.instructions.iter().enumerate() {
        let emitted_position = emitted_positions[index];
        labels_by_ip[emitted_position] = instruction
            .labels_here
            .iter()
            .map(|label| label.name.clone())
            .collect();

        match &instruction.kind {
            IrInstructionKind::Concrete(opcode) => {
                opcodes.push(opcode.clone());
                source_lines_by_ip.push(instruction.source_line);
            }
            IrInstructionKind::JumpLabel(label) => {
                let (target_position, _) = label_positions.get(label).ok_or_else(|| {
                    format!(
                        "{path}:{}: unknown label {}",
                        instruction.source_line, label
                    )
                })?;
                let jump_position = emitted_positions[index] + 1;
                let delta = *target_position as i64 - jump_position as i64 - 1;
                let delta = i32::try_from(delta).map_err(|_| {
                    format!(
                        "{path}:{}: jump delta to {} exceeds i32 range",
                        instruction.source_line, label
                    )
                })?;
                opcodes.push(Opcode::I32Push(delta));
                opcodes.push(Opcode::Jump);
                source_lines_by_ip.push(instruction.source_line);
                source_lines_by_ip.push(instruction.source_line);
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
                let delta = i32::try_from(delta).map_err(|_| {
                    format!(
                        "{path}:{}: jump delta to {} exceeds i32 range",
                        instruction.source_line, label
                    )
                })?;
                opcodes.push(Opcode::I32Push(delta));
                opcodes.push(Opcode::JumpIfTrue);
                source_lines_by_ip.push(instruction.source_line);
                source_lines_by_ip.push(instruction.source_line);
            }
        }
    }

    Ok(AssembledProgram {
        opcodes,
        metadata: ProgramMetadata {
            labels_by_ip,
            source_lines_by_ip,
        },
    })
}

fn emitted_width(kind: &IrInstructionKind) -> usize {
    match kind {
        IrInstructionKind::Concrete(_) => 1,
        IrInstructionKind::JumpLabel(_) | IrInstructionKind::JumpIfTrueLabel(_) => 2,
    }
}
