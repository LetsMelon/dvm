use crate::opcode::Opcode;

pub(crate) struct ParsedProgram {
    pub(crate) items: Vec<ParsedItem>,
}

pub(crate) enum ParsedItem {
    Label(LabelDecl),
    Instruction(ParsedInstruction),
}

#[derive(Clone)]
pub(crate) struct LabelDecl {
    pub(crate) name: String,
    pub(crate) source_line: usize,
}

pub(crate) struct ParsedInstruction {
    pub(crate) kind: ParsedInstructionKind,
    pub(crate) source_line: usize,
}

pub(crate) enum ParsedInstructionKind {
    Concrete(Opcode),
    JumpLabel(String),
    JumpIfTrueLabel(String),
}

pub(crate) struct IrProgram {
    pub(crate) instructions: Vec<IrInstruction>,
}

impl IrProgram {
    // Pass scaffolding uses this helper today in tests and will use it in future real passes.
    #[allow(dead_code)]
    pub(crate) fn remove_instruction(&mut self, index: usize) -> Result<(), String> {
        if index >= self.instructions.len() {
            return Err(format!("instruction index {index} out of bounds"));
        }

        let mut removed = self.instructions.remove(index);
        if removed.labels_here.is_empty() {
            return Ok(());
        }

        let Some(next_instruction) = self.instructions.get_mut(index) else {
            let label = &removed.labels_here[0];
            return Err(format!(
                "cannot remove final instruction because label {} from source line {} would become dangling",
                label.name, label.source_line
            ));
        };

        removed
            .labels_here
            .append(&mut next_instruction.labels_here);
        next_instruction.labels_here = removed.labels_here;
        Ok(())
    }
}

pub(crate) struct IrInstruction {
    pub(crate) labels_here: Vec<LabelDecl>,
    pub(crate) kind: IrInstructionKind,
    pub(crate) source_line: usize,
}

pub(crate) enum IrInstructionKind {
    Concrete(Opcode),
    JumpLabel(String),
    JumpIfTrueLabel(String),
}

pub(crate) fn lower_to_ir(path: &str, parsed: ParsedProgram) -> Result<IrProgram, String> {
    let mut instructions = Vec::new();
    let mut pending_labels = Vec::new();

    for item in parsed.items {
        match item {
            ParsedItem::Label(label) => pending_labels.push(label),
            ParsedItem::Instruction(instr) => {
                let kind = match instr.kind {
                    ParsedInstructionKind::Concrete(opcode) => IrInstructionKind::Concrete(opcode),
                    ParsedInstructionKind::JumpLabel(label) => IrInstructionKind::JumpLabel(label),
                    ParsedInstructionKind::JumpIfTrueLabel(label) => {
                        IrInstructionKind::JumpIfTrueLabel(label)
                    }
                };

                instructions.push(IrInstruction {
                    labels_here: std::mem::take(&mut pending_labels),
                    kind,
                    source_line: instr.source_line,
                });
            }
        }
    }

    if let Some(label) = pending_labels.first() {
        return Err(format!(
            "{path}:{}: label {} does not point to an instruction",
            label.source_line, label.name
        ));
    }

    if instructions.is_empty() {
        return Err(format!("program file {path} did not contain any opcodes"));
    }

    Ok(IrProgram { instructions })
}
