use crate::{
    frontend::ir::{IrInstructionKind, IrProgram},
    opcode::Opcode,
};

use super::OptimizationPass;

pub(crate) struct ConstantFoldMathPass;

impl OptimizationPass for ConstantFoldMathPass {
    fn run(&self, program: &mut IrProgram) -> Result<(), String> {
        let mut index = 0;

        while index + 2 < program.instructions.len() {
            if !program.instructions[index + 1].labels_here.is_empty()
                || !program.instructions[index + 2].labels_here.is_empty()
            {
                index += 1;
                continue;
            }

            let lhs = const_value(&program.instructions[index].kind);
            let rhs = const_value(&program.instructions[index + 1].kind);
            let Some((lhs, rhs)) = lhs.zip(rhs) else {
                index += 1;
                continue;
            };

            let Some(result) = fold_math(&program.instructions[index + 2].kind, lhs, rhs) else {
                index += 1;
                continue;
            };

            let source_line = program.instructions[index].source_line;
            program.instructions[index].kind = IrInstructionKind::Concrete(Opcode::I32Push(result));
            program.instructions[index].source_line = source_line;
            program.remove_instruction(index + 2)?;
            program.remove_instruction(index + 1)?;
            index = index.saturating_sub(2);
        }

        Ok(())
    }
}

fn const_value(kind: &IrInstructionKind) -> Option<i32> {
    match kind {
        IrInstructionKind::Concrete(Opcode::I32Push(value)) => Some(*value),
        IrInstructionKind::Concrete(Opcode::I32Zero) => Some(0),
        _ => None,
    }
}

fn fold_math(kind: &IrInstructionKind, lhs: i32, rhs: i32) -> Option<i32> {
    match kind {
        IrInstructionKind::Concrete(Opcode::I32Add) => Some(lhs.wrapping_add(rhs)),
        IrInstructionKind::Concrete(Opcode::I32Mul) => Some(lhs.wrapping_mul(rhs)),
        IrInstructionKind::Concrete(Opcode::I32Sub) => Some(lhs.wrapping_sub(rhs)),
        IrInstructionKind::Concrete(Opcode::I32Div) if rhs != 0 => Some(lhs.wrapping_div(rhs)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::{
        assembler::assemble_ir, ir::lower_to_ir, parser::parse_source, passes::optimize_ir,
    };

    use super::*;

    fn optimize(source: &str) -> Vec<Opcode> {
        let parsed = parse_source("<test>", source).unwrap();
        let mut ir = lower_to_ir("<test>", parsed).unwrap();
        let pass = ConstantFoldMathPass;
        optimize_ir(&mut ir, &[&pass]).unwrap();
        assemble_ir("<test>", &ir).unwrap()
    }

    #[test]
    fn folds_simple_constant_math() {
        let opcodes = optimize(
            "\
i32.PUSH 3
i32.PUSH 4
i32.ADD
Halt
",
        );

        assert_eq!(opcodes, vec![Opcode::I32Push(7), Opcode::Halt]);
    }

    #[test]
    fn folds_zero_and_push_constants() {
        let opcodes = optimize(
            "\
i32.ZERO
i32.PUSH 4
i32.SUB
Halt
",
        );

        assert_eq!(opcodes, vec![Opcode::I32Push(-4), Opcode::Halt]);
    }

    #[test]
    fn folds_chained_math() {
        let opcodes = optimize(
            "\
i32.PUSH 3
i32.PUSH 4
i32.ADD
i32.PUSH 5
i32.MUL
Halt
",
        );

        assert_eq!(opcodes, vec![Opcode::I32Push(35), Opcode::Halt]);
    }

    #[test]
    fn folds_numeric_programs_when_safe() {
        let opcodes = optimize(
            "\
i32.PUSH 3
i32.PUSH 4
i32.SUB
Halt
",
        );

        assert_eq!(opcodes, vec![Opcode::I32Push(-1), Opcode::Halt]);
    }

    #[test]
    fn does_not_fold_across_stack_change() {
        let opcodes = optimize(
            "\
i32.PUSH 3
i32.PUSH 4
Dup
i32.ADD
Halt
",
        );

        assert_eq!(
            opcodes,
            vec![
                Opcode::I32Push(3),
                Opcode::I32Push(4),
                Opcode::Dup,
                Opcode::I32Add,
                Opcode::Halt
            ]
        );
    }

    #[test]
    fn does_not_fold_across_labels() {
        let opcodes = optimize(
            "\
i32.PUSH 3
.mid
i32.PUSH 4
i32.ADD
Halt
",
        );

        assert_eq!(
            opcodes,
            vec![
                Opcode::I32Push(3),
                Opcode::I32Push(4),
                Opcode::I32Add,
                Opcode::Halt
            ]
        );
    }

    #[test]
    fn does_not_fold_division_by_zero() {
        let opcodes = optimize(
            "\
i32.PUSH 3
i32.ZERO
i32.DIV
Halt
",
        );

        assert_eq!(
            opcodes,
            vec![
                Opcode::I32Push(3),
                Opcode::I32Zero,
                Opcode::I32Div,
                Opcode::Halt
            ]
        );
    }
}
