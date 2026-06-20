use std::collections::HashMap;

use crate::opcode::Opcode;

pub fn compile_source(path: &str, source: &str) -> Result<Vec<Opcode>, String> {
    let parsed = parse_source(path, source)?;
    let mut ir = lower_to_ir(path, parsed)?;
    let noop = NoopOptimizationPass;
    optimize_ir(&mut ir, &[&noop])?;
    assemble_ir(path, &ir)
}

struct ParsedProgram {
    items: Vec<ParsedItem>,
}

enum ParsedItem {
    Label(LabelDecl),
    Instruction(ParsedInstruction),
}

#[derive(Clone)]
struct LabelDecl {
    name: String,
    source_line: usize,
}

struct ParsedInstruction {
    kind: ParsedInstructionKind,
    source_line: usize,
}

enum ParsedInstructionKind {
    Concrete(Opcode),
    JumpLabel(String),
    JumpIfTrueLabel(String),
}

struct IrProgram {
    instructions: Vec<IrInstruction>,
}

impl IrProgram {
    // Pass scaffolding uses this helper today in tests and will use it in future real passes.
    #[allow(dead_code)]
    fn remove_instruction(&mut self, index: usize) -> Result<(), String> {
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

        removed.labels_here.append(&mut next_instruction.labels_here);
        next_instruction.labels_here = removed.labels_here;
        Ok(())
    }
}

struct IrInstruction {
    labels_here: Vec<LabelDecl>,
    kind: IrInstructionKind,
    source_line: usize,
}

enum IrInstructionKind {
    Concrete(Opcode),
    JumpLabel(String),
    JumpIfTrueLabel(String),
}

trait OptimizationPass {
    fn run(&self, program: &mut IrProgram) -> Result<(), String>;
}

struct NoopOptimizationPass;

impl OptimizationPass for NoopOptimizationPass {
    fn run(&self, _program: &mut IrProgram) -> Result<(), String> {
        Ok(())
    }
}

fn optimize_ir(program: &mut IrProgram, passes: &[&dyn OptimizationPass]) -> Result<(), String> {
    for pass in passes {
        pass.run(program)?;
    }

    Ok(())
}

fn parse_source(path: &str, source: &str) -> Result<ParsedProgram, String> {
    let mut items = Vec::new();

    for (line_idx, raw_line) in source.lines().enumerate() {
        let source_line = line_idx + 1;
        let line = strip_comments(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('.') {
            let label = parse_label_decl(path, line, source_line)?;
            items.push(ParsedItem::Label(label));
            continue;
        }

        if let Some(target) = parse_symbolic_jump(path, line, source_line, "JumpIfTrue")? {
            items.push(ParsedItem::Instruction(ParsedInstruction {
                kind: ParsedInstructionKind::JumpIfTrueLabel(target),
                source_line,
            }));
            continue;
        }

        if let Some(target) = parse_symbolic_jump(path, line, source_line, "Jump")? {
            items.push(ParsedItem::Instruction(ParsedInstruction {
                kind: ParsedInstructionKind::JumpLabel(target),
                source_line,
            }));
            continue;
        }

        let opcode = line
            .parse::<Opcode>()
            .map_err(|e| format!("{path}:{source_line}: {e}"))?;

        items.push(ParsedItem::Instruction(ParsedInstruction {
            kind: ParsedInstructionKind::Concrete(opcode),
            source_line,
        }));
    }

    Ok(ParsedProgram { items })
}

fn strip_comments(line: &str) -> &str {
    if let Some(comment_idx) = line.find("//") {
        &line[..comment_idx]
    } else {
        line
    }
}

fn parse_label_decl(path: &str, line: &str, source_line: usize) -> Result<LabelDecl, String> {
    if line.split_whitespace().count() != 1 {
        return Err(format!(
            "{path}:{source_line}: labels must be standalone lines"
        ));
    }

    validate_label_name(path, line, source_line)?;

    Ok(LabelDecl {
        name: line.to_string(),
        source_line,
    })
}

fn parse_symbolic_jump(
    path: &str,
    line: &str,
    source_line: usize,
    opcode: &str,
) -> Result<Option<String>, String> {
    let Some(rest) = line.strip_prefix(opcode) else {
        return Ok(None);
    };

    let rest = rest.trim_start();
    if rest.is_empty() || !rest.starts_with('.') {
        return Ok(None);
    }

    let mut parts = rest.split_whitespace();
    let target = parts.next().expect("split_whitespace produced no first item");
    if let Some(extra) = parts.next() {
        return Err(format!(
            "{path}:{source_line}: unexpected extra argument for {opcode}: {extra}"
        ));
    }

    validate_label_name(path, target, source_line)?;
    Ok(Some(target.to_string()))
}

fn validate_label_name(path: &str, label: &str, source_line: usize) -> Result<(), String> {
    let Some(rest) = label.strip_prefix('.') else {
        return Err(format!(
            "{path}:{source_line}: invalid label {label}; labels must start with '.'"
        ));
    };

    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return Err(format!(
            "{path}:{source_line}: invalid label {label}; labels must match .[A-Za-z_][A-Za-z0-9_]*"
        ));
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!(
            "{path}:{source_line}: invalid label {label}; labels must match .[A-Za-z_][A-Za-z0-9_]*"
        ));
    }

    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        return Err(format!(
            "{path}:{source_line}: invalid label {label}; labels must match .[A-Za-z_][A-Za-z0-9_]*"
        ));
    }

    Ok(())
}

fn lower_to_ir(path: &str, parsed: ParsedProgram) -> Result<IrProgram, String> {
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

fn assemble_ir(path: &str, ir: &IrProgram) -> Result<Vec<Opcode>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct RemoveInstructionPass {
        index: usize,
    }

    impl OptimizationPass for RemoveInstructionPass {
        fn run(&self, program: &mut IrProgram) -> Result<(), String> {
            program.remove_instruction(self.index)
        }
    }

    #[test]
    fn compiles_forward_symbolic_jump() {
        let opcodes = compile_source(
            "<test>",
            "\
Jump .done
Zero
.done
Halt
",
        )
        .unwrap();

        assert_eq!(
            opcodes,
            vec![
                Opcode::PushIntermediate(1),
                Opcode::Jump,
                Opcode::Zero,
                Opcode::Halt
            ]
        );
    }

    #[test]
    fn compiles_backward_symbolic_jump() {
        let opcodes = compile_source(
            "<test>",
            "\
.loop
Zero
Jump .loop
",
        )
        .unwrap();

        assert_eq!(
            opcodes,
            vec![
                Opcode::Zero,
                Opcode::PushIntermediate(-3),
                Opcode::Jump
            ]
        );
    }

    #[test]
    fn rejects_duplicate_labels() {
        let error = compile_source(
            "<test>",
            "\
.loop
Zero
.loop
Halt
",
        )
        .unwrap_err();

        assert!(error.contains("duplicate label .loop"));
    }

    #[test]
    fn rejects_unknown_labels() {
        let error = compile_source("<test>", "Jump .missing\n").unwrap_err();

        assert!(error.contains("unknown label .missing"));
    }

    #[test]
    fn rejects_labels_without_target_instruction() {
        let error = compile_source("<test>", ".dangling\n").unwrap_err();

        assert!(error.contains("label .dangling does not point to an instruction"));
    }

    #[test]
    fn preserves_numeric_programs() {
        let opcodes = compile_source(
            "<test>",
            "\
PushIntermediate 3
PushIntermediate 4
Sub
Halt
",
        )
        .unwrap();

        assert_eq!(
            opcodes,
            vec![
                Opcode::PushIntermediate(3),
                Opcode::PushIntermediate(4),
                Opcode::Sub,
                Opcode::Halt
            ]
        );
    }

    #[test]
    fn noop_pipeline_matches_direct_assembly() {
        let parsed = parse_source(
            "<test>",
            "\
.loop
Zero
Jump .loop
",
        )
        .unwrap();
        let mut ir = lower_to_ir("<test>", parsed).unwrap();
        let direct = assemble_ir("<test>", &ir).unwrap();

        let noop = NoopOptimizationPass;
        optimize_ir(&mut ir, &[&noop]).unwrap();
        let optimized = assemble_ir("<test>", &ir).unwrap();

        assert_eq!(direct, optimized);
    }

    #[test]
    fn relocates_labels_when_pass_removes_instruction() {
        let parsed = parse_source(
            "<test>",
            "\
.loop
Noop
Jump .loop
",
        )
        .unwrap();
        let mut ir = lower_to_ir("<test>", parsed).unwrap();
        let pass = RemoveInstructionPass { index: 0 };
        optimize_ir(&mut ir, &[&pass]).unwrap();
        let opcodes = assemble_ir("<test>", &ir).unwrap();

        assert_eq!(
            opcodes,
            vec![Opcode::PushIntermediate(-2), Opcode::Jump]
        );
    }

    #[test]
    fn rejects_removing_final_labeled_instruction() {
        let parsed = parse_source(
            "<test>",
            "\
Zero
.done
Halt
",
        )
        .unwrap();
        let mut ir = lower_to_ir("<test>", parsed).unwrap();
        let pass = RemoveInstructionPass { index: 1 };
        let error = optimize_ir(&mut ir, &[&pass]).unwrap_err();

        assert!(error.contains("cannot remove final instruction"));
        assert!(error.contains(".done"));
    }
}
