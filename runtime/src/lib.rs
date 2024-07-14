// Runtime should must be connected to the database in order to create the appropriate state provider based on the execution context

// Should be able to 'await' on the individual task execution, so that we can use it for `fee estimation` and `simulation` purposes

// The runtime should handle bookkeeping of what tenant is occupying which thread, so that when a new task belong the same tenant is
// spawned, it can be assigned to that worker thread.

// the runtime isn't restricted to only allowing fixed number of tenants, it can be dynamic, but the number of worker threads should
// be fixed depending on the resource allocations. so as to ntot oversubsribe the resources.

// tenant id should be based on the deployment name, eg the k8s namespace

// current implementation is only limited to performing like instant mining mode in normal Katana.

mod block_producer;
mod context;
mod state;
mod task;

use anyhow::Result;
use block_producer::BlockProducer;
use context::Ctx;
use futures_util::future::FutureExt;
use katana_core::backend::Backend;
use katana_core::utils::get_current_timestamp;
use katana_executor::implementation::blockifier::BlockifierFactory;
use katana_executor::{ExecutorFactory, ExecutorResult, SimulationFlag};
use katana_primitives::block::{ExecutableBlock, PartialHeader};
use katana_primitives::env::CfgEnv;
use katana_primitives::transaction::ExecutableTxWithHash;
use katana_primitives::version::CURRENT_STARKNET_VERSION;
use katana_provider::traits::block::BlockHashProvider;
use katana_provider::traits::env::BlockEnvProvider;
use katana_provider::traits::state::StateFactoryProvider;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::thread::{self, Thread};
use task::AddTask;
use tokio::sync::oneshot;

type ExecutionTaskResult = ExecutorResult<()>;

pub struct JoinHandle<T>(oneshot::Receiver<T>);

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx).map(Result::unwrap)
    }
}

// represents the task to be executed by the worker threads
struct WorkerTask<T> {
    // the execution context based on the specific tenant
    ctx: Ctx,
    transactions: Vec<ExecutableTxWithHash>,
    // the channel to send back the result to the join handle
    _result: oneshot::Sender<T>,
}

struct Inner {
    /// The available worker-threads pool. a thread is removed from the pool
    /// when it is busy executing a task.
    pool: Mutex<Vec<Thread>>,
    backends: Mutex<HashMap<u8, Arc<Backend<BlockifierFactory>>>>,
    pending_tasks: Mutex<VecDeque<WorkerTask<ExecutionTaskResult>>>,
    block_producer: BlockProducer,
}

pub struct Runtime {
    inner: Arc<Inner>,
}

impl Runtime {
    pub fn new(
        worker_threads: usize,
        backends: HashMap<u8, Arc<Backend<BlockifierFactory>>>,
    ) -> Self {
        let block_producer = BlockProducer::new(backends.clone());

        let inner = Arc::new(Inner {
            block_producer,
            pool: Default::default(),
            backends: Mutex::new(backends),
            pending_tasks: Default::default(),
        });

        for i in 0..worker_threads {
            let inner = inner.clone();

            thread::Builder::new()
                .name(format!("rt-worker-thread-{i}"))
                .spawn(move || {
                    loop {
                        while let Some(task) = inner.pending_tasks.lock().pop_back() {
                            let WorkerTask {
                                ctx,
                                transactions,
                                _result,
                            } = task;

                            // get the tenant's db provider
                            let lock = inner.backends.lock();
                            let backend =
                                lock.get(&ctx.tenant).expect("tenant's backend not found");
                            let provider = backend.blockchain.provider();

                            // create execution context
                            let latest_hash = provider.latest_hash().unwrap();
                            let latest_state = provider.latest().unwrap();
                            let mut block_env =
                                provider.block_env_at(latest_hash.into()).unwrap().unwrap();

                            // update block env
                            block_env.number += 1;
                            block_env.timestamp = get_current_timestamp().as_secs();

                            let mut executor = backend.executor_factory.with_state(latest_state);

                            let block = ExecutableBlock {
                                body: transactions,
                                header: PartialHeader {
                                    parent_hash: latest_hash,
                                    number: block_env.number,
                                    timestamp: block_env.timestamp,
                                    version: CURRENT_STARKNET_VERSION,
                                    gas_prices: block_env.l1_gas_prices.clone(),
                                    sequencer_address: block_env.sequencer_address,
                                },
                            };

                            let result = executor.execute_block(block);
                            let output = executor
                                .take_execution_output()
                                .expect("failed to get execution output");

                            // send the execution output to the block producer
                            inner
                                .block_producer
                                .add_block_production_task(ctx.tenant, block_env, output);

                            // send the execution result to the join handle
                            _result.send(result).unwrap();
                        }

                        let handle = thread::current();
                        inner.pool.lock().push(handle);
                        thread::park();
                    }
                })
                .expect("failed to create worker thread");

            tracing::info!(thread = %i, "Worker thread started");
        }

        Runtime { inner }
    }

    pub fn spawn(&self, task: AddTask) -> JoinHandle<ExecutionTaskResult> {
        let (_result, rx) = oneshot::channel();

        // construct the worker task from the input task
        let task = WorkerTask {
            _result,
            ctx: Ctx::default(),
            transactions: task.transactions,
        };

        // push task to pending tasks queue
        self.inner.pending_tasks.lock().push_front(task);
        // wake up a thread to execute the task
        self.wake();

        JoinHandle(rx)
    }

    // TODO: if there is a worker thread that is execuing a task for the same tenant, we should
    // wake up only that thread so that it can pick up the new task.
    fn wake(&self) {
        // unpark an available thread from the pool
        if let Some(thread) = self.inner.pool.lock().pop() {
            thread.unpark();
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use katana_core::{
        backend::config::StarknetConfig,
        constants::{DEFAULT_INVOKE_MAX_STEPS, DEFAULT_VALIDATE_MAX_STEPS, MAX_RECURSION_DEPTH},
        env::get_default_vm_resource_fee_cost,
    };
    use katana_executor::implementation::blockifier::BlockifierFactory;
    use katana_primitives::felt::FieldElement;
    use katana_primitives::{
        chain::ChainId,
        env::{CfgEnv, FeeTokenAddressses},
        genesis::constant::DEFAULT_FEE_TOKEN_ADDRESS,
        transaction::{InvokeTx, InvokeTxV1},
    };
    use starknet::macros::{felt, selector};
    use tracing_subscriber::{fmt, EnvFilter};

    fn tx() -> ExecutableTxWithHash {
        let invoke = InvokeTx::V1(InvokeTxV1 {
            sender_address: felt!("0x1").into(),
            calldata: vec![
                DEFAULT_FEE_TOKEN_ADDRESS.into(),
                selector!("transfer"),
                FieldElement::THREE,
                felt!("0x100"),
                FieldElement::ONE,
                FieldElement::ZERO,
            ],
            max_fee: 10_000,
            ..Default::default()
        });

        ExecutableTxWithHash::new(invoke.into())
    }

    async fn create_tenants_backend() -> HashMap<u8, Arc<Backend<BlockifierFactory>>> {
        let mut backends = HashMap::new();

        let chains = [
            ChainId::MAINNET,
            ChainId::SEPOLIA,
            ChainId::GOERLI,
            ChainId::parse("KATANA").unwrap(),
        ];

        for i in 0..4 {
            let cfg_env = CfgEnv {
                chain_id: chains[i],
                vm_resource_fee_cost: get_default_vm_resource_fee_cost(),
                invoke_tx_max_n_steps: DEFAULT_INVOKE_MAX_STEPS,
                validate_max_n_steps: DEFAULT_VALIDATE_MAX_STEPS,
                max_recursion_depth: MAX_RECURSION_DEPTH,
                fee_token_addresses: FeeTokenAddressses {
                    eth: DEFAULT_FEE_TOKEN_ADDRESS,
                    strk: Default::default(),
                },
            };

            let mut starknet_config = StarknetConfig::default();
            starknet_config.db_dir = Some(format!("./db/{i}").into());

            let flag = SimulationFlag {
                skip_fee_transfer: true,
                skip_validate: true,
                ..Default::default()
            };

            let executor_factory = BlockifierFactory::new(cfg_env, flag);
            let backend = Backend::new(Arc::new(executor_factory), starknet_config).await;
            backends.insert(i as u8, Arc::new(backend));
        }

        backends
    }

    const DEFAULT_LOG_FILTER: &str = "info,executor=trace,forking::backend=trace,server=debug,\
                                      katana_core=trace,blockifier=off,jsonrpsee_server=off,\
                                      hyper=off,messaging=debug,node=error";

    #[tokio::test]
    async fn main() -> anyhow::Result<()> {
        let subscriber = fmt::Subscriber::builder()
            .with_env_filter(
                EnvFilter::try_from_default_env().or(EnvFilter::try_new(DEFAULT_LOG_FILTER))?,
            )
            .finish();

        tracing::subscriber::set_global_default(subscriber)?;

        // Configure the runtiem with the specified number of worker threads and tenants
        // This basically means that the runtime will have 4 worker threads and will be able to handle requests from 4 different tenants
        // in parallel.
        let backends = create_tenants_backend().await;
        let runtime = Runtime::new(4, backends);

        let handle = runtime.spawn(AddTask {
            tenant_id: 0,
            transactions: vec![tx()],
        });
        // let _ = runtime.spawn(AddTask::default());
        // let _ = runtime.spawn(AddTask::default());

        let result = handle.await;

        println!("Result: {result:?}");

        Ok(())
    }
}
