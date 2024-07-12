use katana_executor::SimulationFlag;
use katana_primitives::env::CfgEnv;
use katana_provider::traits::state::StateProvider;

/// The task execution context.
pub struct Ctx<P>
where
    P: StateProvider,
{
    // the tenant id of this context
    tenant: u8,
    // state provider
    provider: P,
    /// execution flags for the executor
    execution_flags: SimulationFlag,
    // the chain configuration
    cfg: CfgEnv,
}
