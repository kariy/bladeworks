use katana_executor::SimulationFlag;
use katana_primitives::transaction::ExecutableTxWithHash;

enum Task {
    Add,
    Simulation(SimulationTask),
}

struct SimulationTask {
    tenant_id: u64,
    flags: SimulationFlag,
    transactions: Vec<ExecutableTxWithHash>,
}
