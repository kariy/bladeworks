mod context;
mod task;

use bladeworks_db::Db;
use katana_db::mdbx;
use katana_executor::implementation::noop::NoopExecutorFactory;
use katana_executor::ExecutorFactory;
use katana_primitives::env::BlockEnv;
use katana_primitives::transaction::ExecutableTxWithHash;
use katana_provider::traits::state::StateFactoryProvider;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::thread::{self, sleep, Thread};
use tokio::sync::oneshot;

pub struct JoinHandle {
    rx: oneshot::Receiver<()>,
}

struct Task {
    message: String,
    tx: oneshot::Sender<()>,
}

struct Inner {
    db: Db<mdbx::DbEnv>,
    pool: Mutex<Vec<Thread>>,
    pending_tasks: Mutex<VecDeque<Task>>,
}

pub struct Runtime {
    // database: Db,
    inner: Arc<Inner>,
}

impl Runtime {
    pub fn new(worker_threads: usize) -> Self {
        let inner = Arc::new(Inner {
            pool: Default::default(),
            pending_tasks: Default::default(),
            db: Db::init("./db").expect("failed to initialize database"),
        });

        for _ in 0..worker_threads {
            let inner = inner.clone();

            thread::spawn(move || {
                loop {
                    while let Some(task) = inner.pending_tasks.lock().pop_back() {
                        // create execution context
                        let provider = inner.db.provider(0).unwrap().unwrap();
                        let block_env = BlockEnv::default();
                        let state = provider.latest().unwrap();

                        // execute tasks
                        let factory = NoopExecutorFactory::new();
                        let mut executor = factory.with_state_and_block_env(state, block_env);
                        executor.execute_transactions(Vec::new()).unwrap();

                        // send back the execution result
                        task.tx.send(()).unwrap();
                    }

                    let handle = thread::current();
                    inner.pool.lock().push(handle);
                    thread::park();
                }
            });
        }

        Runtime { inner }
    }

    pub fn spawn(&self, message: &str) -> JoinHandle {
        let (tx, rx) = oneshot::channel();

        // create the task
        let task = Task {
            tx,
            message: message.to_string(),
        };
        // push task to pending tasks queue
        self.inner.pending_tasks.lock().push_front(task);
        self.wake();

        JoinHandle { rx }
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
    let handle1 = runtime.spawn("hello");
    let handle2 = runtime.spawn("world");
    let handle3 = runtime.spawn("!");
    let handle4 = runtime.spawn("4th message");
    let handle5 = runtime.spawn("5th message");
    sleep(Duration::from_secs(5));
}

// runtime should must be connected to the database in order to create the appropriate state provider based on the execution context
// should be able to 'await' on the individual task execution, so that we can use it for `fee estimation` and `simulation` purposes
