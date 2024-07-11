// use crossbeam_channel::{unbounded, Receiver, Sender};
use katana_executor::implementation::noop::NoopExecutorFactory;
use katana_executor::{BlockExecutor, ExecutorFactory, ExecutorResult};
use katana_primitives::transaction::ExecutableTxWithHash;
use katana_provider::traits::state::StateProvider;
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, Thread};
use std::time::Duration;

// represent a single task to be executed by a worker
#[derive(Debug)]
struct Task {
    transactions: Vec<ExecutableTxWithHash>,
    tenant_id: String,
    result_sender: Sender<ExecutionResult>,
}

// represent a single worker thread
#[derive(Debug)]
struct Worker {
    // the tenant id of the task execution context
    current_tenant: Option<String>,
    // the worker queue of tasks to execute.
    // this is gonna be used for queuing incoming tasks when
    // the worker is already handling a tasks of the same tenant.
    task_queue: VecDeque<Task>,
    // the thread handle
    thread: Option<Thread>,
}

#[derive(Debug)]
pub struct ExecutorRuntime {
    workers: Vec<Arc<(Mutex<Worker>, Condvar)>>,
}

#[derive(Debug)]
pub struct ExecutionResult {
    tenant_id: String,
    results: Vec<TransactionResult>,
}

#[derive(Debug)]
pub struct TransactionResult {
    // Add fields as needed, e.g.:
    success: bool,
    error_message: Option<String>,
}

impl ExecutorRuntime {
    pub fn new(num_workers: usize) -> Self {
        let workers: Vec<Arc<(Mutex<Worker>, Condvar)>> = (0..num_workers)
            .map(|_| {
                Arc::new((
                    Mutex::new(Worker {
                        thread: None,
                        current_tenant: None,
                        task_queue: VecDeque::new(),
                    }),
                    Condvar::new(),
                ))
            })
            .collect();

        let workers_clone = workers.clone();

        for worker in &workers {
            let worker = Arc::clone(worker);
            // let executor_factory = Arc::clone(&executor_factory);

            thread::spawn(move || {
                let (lock, cvar) = &*worker;
                loop {
                    let mut worker = lock.lock().unwrap();
                    while worker.task_queue.is_empty() {
                        worker.thread = Some(thread::current());
                        worker = cvar.wait(worker).unwrap();
                    }

                    if let Some(task) = worker.task_queue.pop_front() {
                        worker.current_tenant = Some(task.tenant_id.clone());
                        drop(worker);

                        // Execute task and send results
                        let result = execute_task(&task);
                        task.result_sender.send(result).unwrap();

                        let mut worker = lock.lock().unwrap();
                        worker.current_tenant = None;
                    }
                }
            });
        }

        ExecutorRuntime { workers }
    }

    pub fn spawn(
        &self,
        transactions: Vec<ExecutableTxWithHash>,
        tenant_id: String,
    ) -> Receiver<ExecutionResult> {
        let (sender, receiver) = channel();

        let task = Task {
            tenant_id,
            transactions,
            result_sender: sender,
        };

        // find an available worker or a worker that is already working on the same tenant id
        for worker in &self.workers {
            let (lock, cvar) = &**worker;
            let mut worker = lock.lock().unwrap();

            if worker.current_tenant.as_ref() == Some(&task.tenant_id) {
                worker.task_queue.push_back(task);
                if let Some(thread) = worker.thread.take() {
                    thread.unpark();
                }
                // cvar.notify_one();
                return receiver;
            }
        }

        receiver
    }
}

// Placeholder execution function
fn execute_task(task: &Task) -> ExecutionResult {
    println!("Executing task for tenant: {}", task.tenant_id);
    println!("Number of transactions: {}", task.transactions.len());

    // Simulate some work
    thread::sleep(Duration::from_millis(100 * task.transactions.len() as u64));

    // Simulate execution results
    let results = task
        .transactions
        .iter()
        .map(|_| TransactionResult {
            success: true,
            error_message: None,
        })
        .collect();

    println!("Task completed for tenant: {}", task.tenant_id);

    ExecutionResult {
        tenant_id: task.tenant_id.clone(),
        results,
    }
}

// Usage example
fn main() {
    // let executor_factory = Arc::new(DummyExecutorFactory);
    let runtime = ExecutorRuntime::new(4);

    // Simulate some transactions
    let transactions = vec![];
    let tenant_id = String::from("tenant1");
    let receiver = runtime.spawn(transactions, tenant_id);

    // // Add more tasks to test the runtime
    // let receiver2 = runtime.spawn(
    //     vec![ExecutableTxWithHash::default(); 3],
    //     "tenant2".to_string(),
    // );
    // let receiver3 = runtime.spawn(
    //     vec![ExecutableTxWithHash::default(); 7],
    //     "tenant1".to_string(),
    // );
    // let receiver4 = runtime.spawn(
    //     vec![ExecutableTxWithHash::default(); 2],
    //     "tenant3".to_string(),
    // );

    // // Process results
    // for receiver in [receiver, receiver2, receiver3, receiver4] {
    //     thread::spawn(move || {
    //         if let Ok(result) = receiver.recv() {
    //             println!(
    //                 "Received result for tenant {}: {:?}",
    //                 result.tenant_id, result
    //             );
    //             // Here you would send the result to another service for processing
    //         }
    //     });
    // }

    // Keep the main thread alive to observe the execution
    thread::sleep(Duration::from_secs(5));
}

// // Dummy implementations for testing
// #[derive(Clone, Default)]
// struct DummyExecutorFactory;

// impl ExecutorFactory for DummyExecutorFactory {
//     fn with_state<'a, P>(&self, _state: P) -> Box<dyn BlockExecutor<'a> + 'a>
//     where
//         P: StateProvider + 'a,
//     {
//         Box::new(DummyBlockExecutor)
//     }

//     // Implement other required methods...
// }

// struct DummyBlockExecutor;

// impl<'a> BlockExecutor<'a> for DummyBlockExecutor {
//     fn execute_transactions(
//         &mut self,
//         _transactions: Vec<ExecutableTxWithHash>,
//     ) -> ExecutorResult<()> {
//         Ok(())
//     }

//     // Implement other required methods...
// }

// impl ExecutableTxWithHash {
//     fn default() -> Self {
//         // Implement a default ExecutableTxWithHash for testing
//         unimplemented!("Implement default ExecutableTxWithHash")
//     }
// }
