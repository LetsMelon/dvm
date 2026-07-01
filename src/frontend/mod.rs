mod assembler;
mod ir;
mod parser;
pub mod passes;

use crate::opcode::Opcode;

use self::{
    assembler::assemble_ir,
    ir::lower_to_ir,
    parser::parse_source,
    passes::{
        constant_fold_f32::ConstantFoldF32Pass, constant_fold_math::ConstantFoldMathPass,
        noop::NoopOptimizationPass, optimize_ir,
    },
};

pub fn compile_source(path: &str, source: &str) -> Result<Vec<Opcode>, String> {
    let parsed = parse_source(path, source)?;
    let mut ir = lower_to_ir(path, parsed)?;

    let fold_math = ConstantFoldMathPass;
    let fold_f32 = ConstantFoldF32Pass;
    let noop = NoopOptimizationPass;

    optimize_ir(&mut ir, &[&fold_math, &fold_f32, &noop])?;
    assemble_ir(path, &ir)
}

#[cfg(test)]
mod tests {
    use crate::opcode::Opcode;

    use super::{
        assembler::assemble_ir,
        ir::lower_to_ir,
        parser::parse_source,
        passes::{OptimizationPass, noop::NoopOptimizationPass, optimize_ir},
        *,
    };

    struct RemoveInstructionPass {
        index: usize,
    }

    impl OptimizationPass for RemoveInstructionPass {
        fn run(&self, program: &mut ir::IrProgram) -> Result<(), String> {
            program.remove_instruction(self.index)
        }
    }

    #[test]
    fn compiles_forward_symbolic_jump() {
        let opcodes = compile_source(
            "<test>",
            "\
Jump .done
i32.ZERO
.done
Halt
",
        )
        .unwrap();

        assert_eq!(
            opcodes,
            vec![
                Opcode::I32Push(1),
                Opcode::Jump,
                Opcode::I32Zero,
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
i32.ZERO
Jump .loop
",
        )
        .unwrap();

        assert_eq!(
            opcodes,
            vec![Opcode::I32Zero, Opcode::I32Push(-3), Opcode::Jump]
        );
    }

    #[test]
    fn rejects_duplicate_labels() {
        let error = compile_source(
            "<test>",
            "\
.loop
i32.ZERO
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
    fn noop_pipeline_matches_direct_assembly() {
        let parsed = parse_source(
            "<test>",
            "\
.loop
i32.ZERO
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

        assert_eq!(opcodes, vec![Opcode::I32Push(-2), Opcode::Jump]);
    }

    #[test]
    fn rejects_removing_final_labeled_instruction() {
        let parsed = parse_source(
            "<test>",
            "\
i32.ZERO
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
