pub mod constant_fold_math;
pub mod noop;

use super::ir::IrProgram;

pub(crate) trait OptimizationPass {
    fn run(&self, program: &mut IrProgram) -> Result<(), String>;
}

pub(crate) fn optimize_ir(
    program: &mut IrProgram,
    passes: &[&dyn OptimizationPass],
) -> Result<(), String> {
    for pass in passes {
        pass.run(program)?;
    }

    Ok(())
}
