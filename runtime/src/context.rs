use katana_executor::SimulationFlag;
use katana_primitives::env::CfgEnv;
use katana_provider::traits::state::StateProvider;

/// The task execution context.
#[derive(Debug, Default)]
pub struct Ctx
// <P>
// where
//     P: StateProvider,
{
    // the tenant id of this context
    pub tenant: u8,
    // // state provider
    // pub provider: P,
    // execution flags for the executor
    pub execution_flags: SimulationFlag,
    // the chain configuration
    pub cfg: CfgEnv,
}

/// The tenant deployment configuration.
///
/// This represents the deployment-speficic configuration when creating
/// a new Slot instance.
#[derive(Debug, Default)]
pub struct TenantConfig {
    pub block_time: u64,
}
