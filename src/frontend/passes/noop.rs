use super::OptimizationPass;
use crate::frontend::ir::IrProgram;

pub(crate) struct NoopOptimizationPass;

impl OptimizationPass for NoopOptimizationPass {
    fn run(&self, _program: &mut IrProgram) -> Result<(), String> {
        Ok(())
    }
}
