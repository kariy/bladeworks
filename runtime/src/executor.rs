use std::time::Duration;

use katana_executor::{
    BlockExecutor, ExecutionOutput, ExecutionResult, ExecutorExt, ExecutorResult,
};
use katana_primitives::{
    block::ExecutableBlock,
    env::BlockEnv,
    transaction::{ExecutableTxWithHash, TxWithHash},
};
use katana_provider::traits::state::StateProvider;

pub struct Executor;

impl BlockExecutor<'static> for Executor {
    fn execute_block(&mut self, block: ExecutableBlock) -> ExecutorResult<()> {
        println!("executing block with {} txs", block.body.len());
        Ok(())
    }

    fn execute_transactions(
        &mut self,
        transactions: Vec<ExecutableTxWithHash>,
    ) -> ExecutorResult<()> {
        println!("executing {} txs", transactions.len());
        std::thread::sleep(Duration::from_secs(1));
        Ok(())
    }

    /// Takes the output state of the executor.
    fn take_execution_output(&mut self) -> ExecutorResult<ExecutionOutput> {
        unimplemented!()
    }

    /// Returns the current state of the executor.
    fn state(&self) -> Box<dyn StateProvider + 'static> {
        unimplemented!()
    }

    fn transactions(&self) -> &[(TxWithHash, ExecutionResult)] {
        unimplemented!()
    }

    fn block_env(&self) -> BlockEnv {
        unimplemented!()
    }
}

impl ExecutorExt for Executor {
    fn call(
        &self,
        call: katana_executor::EntryPointCall,
    ) -> Result<Vec<katana_primitives::FieldElement>, katana_executor::ExecutionError> {
        unimplemented!()
    }

    fn estimate_fee(
        &self,
        transactions: Vec<ExecutableTxWithHash>,
        flags: katana_executor::SimulationFlag,
    ) -> Vec<Result<katana_primitives::fee::TxFeeInfo, katana_executor::ExecutionError>> {
        unimplemented!()
    }

    fn simulate(
        &self,
        transactions: Vec<ExecutableTxWithHash>,
        flags: katana_executor::SimulationFlag,
    ) -> Vec<katana_executor::ResultAndStates> {
        unimplemented!()
    }
}
