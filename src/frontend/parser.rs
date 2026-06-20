use crate::opcode::Opcode;

use super::ir::{LabelDecl, ParsedInstruction, ParsedInstructionKind, ParsedItem, ParsedProgram};

pub(crate) fn parse_source(path: &str, source: &str) -> Result<ParsedProgram, String> {
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
    let target = parts
        .next()
        .expect("split_whitespace produced no first item");
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
