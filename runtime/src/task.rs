use katana_executor::SimulationFlag;
use katana_primitives::transaction::ExecutableTxWithHash;

pub enum Task {
    Add(AddTask),
    Simulation(SimulationTask),
}

#[derive(Debug, Default)]
pub struct AddTask {
    pub tenant_id: u64,
    pub transactions: Vec<ExecutableTxWithHash>,
}

pub struct SimulationTask {
    pub tenant_id: u64,
    pub flags: SimulationFlag,
    pub transactions: Vec<ExecutableTxWithHash>,
}
