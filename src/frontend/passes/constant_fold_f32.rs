use crate::{
    frontend::ir::{IrInstructionKind, IrProgram},
    opcode::Opcode,
};

use super::OptimizationPass;

pub(crate) struct ConstantFoldF32Pass;

impl OptimizationPass for ConstantFoldF32Pass {
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

            let Some(result) = fold_f32(&program.instructions[index + 2].kind, lhs, rhs) else {
                index += 1;
                continue;
            };

            let source_line = program.instructions[index].source_line;
            program.instructions[index].kind = result;
            program.instructions[index].source_line = source_line;
            program.remove_instruction(index + 2)?;
            program.remove_instruction(index + 1)?;
            index = index.saturating_sub(2);
        }

        Ok(())
    }
}

fn const_value(kind: &IrInstructionKind) -> Option<f32> {
    match kind {
        IrInstructionKind::Concrete(Opcode::F32Push(value)) => Some(*value),
        _ => None,
    }
}

fn fold_f32(kind: &IrInstructionKind, lhs: f32, rhs: f32) -> Option<IrInstructionKind> {
    let opcode = match kind {
        IrInstructionKind::Concrete(Opcode::F32Add) => Opcode::F32Push(lhs + rhs),
        IrInstructionKind::Concrete(Opcode::F32Sub) => Opcode::F32Push(lhs - rhs),
        IrInstructionKind::Concrete(Opcode::F32Mul) => Opcode::F32Push(lhs * rhs),
        IrInstructionKind::Concrete(Opcode::F32Div) => Opcode::F32Push(lhs / rhs),
        IrInstructionKind::Concrete(Opcode::F32Eq) => {
            Opcode::I32Push((lhs.to_bits() == rhs.to_bits()) as i32)
        }
        IrInstructionKind::Concrete(Opcode::F32Gt) => Opcode::I32Push((lhs > rhs) as i32),
        IrInstructionKind::Concrete(Opcode::F32Ge) => Opcode::I32Push((lhs >= rhs) as i32),
        _ => return None,
    };

    Some(IrInstructionKind::Concrete(opcode))
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
        let pass = ConstantFoldF32Pass;
        optimize_ir(&mut ir, &[&pass]).unwrap();
        assemble_ir("<test>", &ir).unwrap()
    }

    fn assert_single_f32_push(opcodes: &[Opcode], expected: f32) {
        let [Opcode::F32Push(actual), Opcode::Halt] = opcodes else {
            panic!("expected folded f32 push followed by Halt, got {opcodes:?}");
        };

        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn folds_f32_arithmetic() {
        let opcodes = optimize(
            "\
f32.PUSH 10.5
f32.PUSH 4.25
f32.SUB
Halt
",
        );
        assert_single_f32_push(&opcodes, 6.25);

        let opcodes = optimize(
            "\
f32.PUSH 2.5
f32.PUSH 4
f32.MUL
Halt
",
        );
        assert_single_f32_push(&opcodes, 10.0);

        let opcodes = optimize(
            "\
f32.PUSH 9
f32.PUSH 3
f32.DIV
Halt
",
        );
        assert_single_f32_push(&opcodes, 3.0);
    }

    #[test]
    fn folds_f32_comparisons() {
        let opcodes = optimize(
            "\
f32.PUSH 4.5
f32.PUSH 4.5
f32.EQ
Halt
",
        );
        assert_eq!(opcodes, vec![Opcode::I32Push(1), Opcode::Halt]);

        let opcodes = optimize(
            "\
f32.PUSH 4.5
f32.PUSH 3.5
f32.GT
Halt
",
        );
        assert_eq!(opcodes, vec![Opcode::I32Push(1), Opcode::Halt]);

        let opcodes = optimize(
            "\
f32.PUSH 4.5
f32.PUSH 5
f32.GE
Halt
",
        );
        assert_eq!(opcodes, vec![Opcode::I32Push(0), Opcode::Halt]);
    }

    #[test]
    fn folds_f32_eq_like_i32_eq() {
        let opcodes = optimize(
            "\
f32.PUSH 0
f32.PUSH -0
f32.EQ
Halt
",
        );

        assert_eq!(opcodes, vec![Opcode::I32Push(0), Opcode::Halt]);
    }

    #[test]
    fn does_not_fold_across_labels() {
        let opcodes = optimize(
            "\
f32.PUSH 3
.mid
f32.PUSH 4
f32.ADD
Halt
",
        );

        assert_eq!(
            opcodes,
            vec![
                Opcode::F32Push(3.0),
                Opcode::F32Push(4.0),
                Opcode::F32Add,
                Opcode::Halt
            ]
        );
    }
}
