mod context;
mod executor;
mod task;

use bladeworks_db::Db;
use context::{Ctx, TenantConfig};
use executor::Executor;
use futures_util::future::FutureExt;
use katana_db::mdbx;
use katana_executor::implementation::blockifier::BlockifierFactory;
use katana_executor::implementation::noop::NoopExecutorFactory;
use katana_executor::{BlockExecutor, ExecutorFactory, ExecutorResult};
use katana_primitives::env::BlockEnv;
use katana_primitives::transaction::ExecutableTxWithHash;
use katana_provider::traits::state::{StateFactoryProvider, StateProvider};
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
    db: Db<mdbx::DbEnv>,
    /// The available worker-threads pool. a thread is removed from the pool
    /// when it is busy executing a task.
    pool: Mutex<Vec<Thread>>,
    configs: Mutex<HashMap<u8, TenantConfig>>,
    pending_tasks: Mutex<VecDeque<WorkerTask<ExecutionTaskResult>>>,
}

pub struct Runtime {
    inner: Arc<Inner>,
}

impl Runtime {
    pub fn new(worker_threads: usize) -> Self {
        let inner = Arc::new(Inner {
            pool: Default::default(),
            pending_tasks: Default::default(),
            db: Db::init("./db").expect("failed to initialize database"),
            configs: Default::default(),
        });

        for _ in 0..worker_threads {
            let inner = inner.clone();

            thread::spawn(move || {
                loop {
                    while let Some(task) = inner.pending_tasks.lock().pop_back() {
                        // // create execution context
                        // let provider = inner.db.provider(0).unwrap().unwrap();
                        // let block_env = BlockEnv::default();
                        // let state = provider.latest().unwrap();

                        // // execute tasks
                        // let factory = BlockifierFactory::new(cfg, flags);
                        // let factory = NoopExecutorFactory::new();
                        // let mut executor = factory.with_state_and_block_env(state, block_env);
                        let mut executor = Executor;
                        let result = executor.execute_transactions(task.transactions);

                        // send back the execution result
                        task._result.send(result).unwrap();
                    }

                    let handle = thread::current();
                    inner.pool.lock().push(handle);
                    thread::park();
                }
            });
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

    fn wake(&self) {
        // unpark an available thread from the pool
        if let Some(thread) = self.inner.pool.lock().pop() {
            thread.unpark();
        }
    }
}

#[tokio::test]
async fn main() {
    let runtime = Runtime::new(4);
    let handle = runtime.spawn(AddTask::default());
    let result = handle.await;
}

// Runtime should must be connected to the database in order to create the appropriate state provider based on the execution context

// Should be able to 'await' on the individual task execution, so that we can use it for `fee estimation` and `simulation` purposes

// The runtime should handle bookkeeping of what tenant is occupying which thread, so that when a new task belong the same tenant is
// spawned, it can be assigned to that worker thread.
