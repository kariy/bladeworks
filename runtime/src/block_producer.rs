use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bladeworks_db::Db;
use futures::channel::mpsc::{channel, Receiver, SendError, Sender};
use futures::SinkExt;
use futures_util::{FutureExt, Stream};
use katana_core::backend::Backend;
use katana_core::service::block_producer::{
    BlockProductionError, MinedBlockOutcome, TxWithOutcome,
};
use katana_db::mdbx;
use katana_executor::implementation::blockifier::BlockifierFactory;
use katana_executor::{ExecutionOutput, ExecutionResult};
use katana_primitives::env::BlockEnv;
use katana_primitives::transaction::TxWithHash;
use katana_tasks::{BlockingTaskPool, BlockingTaskResult};
use parking_lot::Mutex;

type KatanaBlockProducer = katana_core::service::block_producer::BlockProducer<BlockifierFactory>;

// The job of the block producer is to produce blocks for a particular tenant based on each tenant's block configuration.
// The block time is the maximum time a block will be opened to accept transactions. A block will be opened in 'pending' mode for the duration of the block time.
// If no block time is specified, the block producer will instantly produce a block from the executed transactions and store it in the database.
//
// when a new transaction is received, the producer will first check if there's an ongoing block production task for the tenant.
// If there is, the transaction will be added to the block. If there isn't, a new block production task will be created.
// The producer will then check if the block time has elapsed. If it has, the block will be produced and stored in the database.
//
// list of possible fields:
// - list of ongoing block production tasks for a particular tenant
// - a handle for receiving the execution result from the worker thread
// - a handle to the database env

// repsonsible for providing the current/historical block context for a tenant.

// #[derive(Debug)]
// pub struct BlockProducer(Sender<()>);

// impl BlockProducer {
//     pub fn new() -> BlockProducer {
//         todo!()
//     }

//     pub fn add_transaction(&self, tenant: u8, transactions: Vec<(TxWithHash, ExecutionResult)>) {
//         self.0.send(()).unwrap();
//     }
// }

struct Request {
    tenant: u8,
    env: BlockEnv,
    execution_output: ExecutionOutput,
}

type ServiceFut<T> = Pin<Box<dyn Future<Output = BlockingTaskResult<T>> + Send + Sync>>;
type Task = ServiceFut<Result<MinedBlockOutcome, BlockProductionError>>;

struct BlockProducerService {
    // receiver-end from the service handle to get the queue tasks
    rx: Receiver<Request>,
    // // database handle
    // db: Db<mdbx::DbEnv>,
    // task pool for spawning blocking tasks
    task_pool: BlockingTaskPool,
    // the katana backend for each tenant
    backends: HashMap<u8, Arc<Backend<BlockifierFactory>>>,
    // tasks that are currently being processed
    ongoing_tasks: Vec<Task>,
    // tasks that are waiting to be processed
    queued_requests: Vec<Request>,
}

pub struct BlockProducer(Mutex<Sender<Request>>);

impl BlockProducer {
    pub fn new(
        // db: Db,
        backends: HashMap<u8, Arc<Backend<BlockifierFactory>>>,
    ) -> Self {
        let (tx, rx) = channel(100);

        // create the block producer service
        let service = BlockProducerService {
            // db,
            rx,
            backends,
            queued_requests: Vec::new(),
            ongoing_tasks: Vec::new(),
            task_pool: BlockingTaskPool::new().unwrap(),
        };

        // create a tokio runtime to execute the service
        let tokio_rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build block producer service tokio runtime");

        // spawn the service in a separate thread
        std::thread::Builder::new()
            .name("block-producer".into())
            .spawn(move || tokio_rt.block_on(service));

        Self(Mutex::new(tx))
    }

    pub fn add_block_production_task(
        &self,
        tenant: u8,
        env: BlockEnv,
        execution_output: ExecutionOutput,
    ) {
        self.0
            .lock()
            .try_send(Request {
                tenant,
                env,
                execution_output,
            })
            .expect("failed to send block production task to block producer service");
    }
}

impl BlockProducerService {
    fn handle_request(&mut self, request: Request) {
        let backend = Arc::clone(self.backends.get(&request.tenant).unwrap());

        // spawn the task in the blocking thread pool
        let fut = self
            .task_pool
            .spawn(move || backend.do_mine_block(&request.env, request.execution_output));

        // push the task to the ongoing tasks so that it can be polled later
        self.ongoing_tasks.push(Box::pin(fut) as Task);
    }
}

impl Future for BlockProducerService {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        loop {
            // receive the incoming requests and inserts them in the task queue
            loop {
                match Pin::new(&mut this.rx).poll_next(cx) {
                    Poll::Ready(Some(req)) => {
                        this.queued_requests.push(req);
                    }

                    // Resolve if stream is exhausted.
                    Poll::Ready(None) => {
                        return Poll::Ready(());
                    }

                    Poll::Pending => {
                        break;
                    }
                }
            }

            // convert the queued requests into tasks
            while let Some(req) = this.queued_requests.pop() {
                this.handle_request(req);
            }

            // poll each of the ongoing tasks
            for n in (0..this.ongoing_tasks.len()).rev() {
                let mut fut = this.ongoing_tasks.swap_remove(n);
                // poll the future and if the future is still pending, push it back to the
                // pending requests so that it will be polled again
                if fut.poll_unpin(cx).is_pending() {
                    this.ongoing_tasks.push(fut);
                }
            }

            // if no queued requests, then yield
            if this.ongoing_tasks.is_empty() {
                return Poll::Pending;
            }
        }
    }
}
