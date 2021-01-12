use crate::{Error, Project, Result};
use crossbeam_channel::{Receiver, Sender};
use std::cell::Cell;
use std::collections::HashMap;
use std::thread;

/// Perform a run over a project
pub fn run(project: &Project, opts: RunOptions) -> Result<()> {
    // TODO:
    // * Gather the initial list of `ServiceStep`
    // * Start workers
    // * Continuously process dependencies on the fly
    // * Send work
    // * Process responses

    Ok(())
}

/// Options for running the project
#[derive(Default)]
pub struct RunOptions {
    /// array of step:service pairs that should be rebuilt as part of this run
    pub service_steps: Vec<String>,
    /// Gather changed services since this commit ID
    pub changed_since: Option<String>,

    /// Whether we should check and run dependencies
    pub run_deps: bool,
    /// Whether we should check and run reverse dependencies
    pub run_rdeps: bool,

    /// Number of threads for the run
    ///
    /// Will default to the number of CPUs
    pub workers: Option<usize>,
}

/// Request message sent to a worker
enum WorkerRequest {
    Stop,
    RunScript(WorkerRunRequest),
}

/// Informations for a `WorkerRequest::RunScript` request
struct WorkerRunRequest {
    pub id: String,
    pub container_image: String,
    pub env: HashMap<String, String>,
    pub script: String,
}

/// Response from a worker to a `WorkerRequest::RunScript` request
enum WorkerResponse {
    Success { id: String },
    Failure { id: String, status_code: i32 },
}

/// Run Worker
struct Worker {
    /// Sender for the request channel
    ///
    /// Used to send requests to the worker
    req_tx: Sender<WorkerRequest>,
    /// Received for the request channel
    ///
    /// Used internally by the worker
    req_rx: Receiver<WorkerRequest>,

    /// Sender for the response channel
    ///
    /// Used internally by the worker
    res_tx: Sender<WorkerResponse>,
    /// Received for the response channel
    ///
    /// Used to receive messages from the worker
    pub res_rx: Receiver<WorkerResponse>,

    join_handle: Cell<Option<thread::JoinHandle<()>>>,
}

impl Worker {
    /// Create a new worker
    pub fn new() -> Self {
        let (req_tx, req_rx) = crossbeam_channel::unbounded::<WorkerRequest>();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<WorkerResponse>();
        Self {
            req_tx,
            req_rx,
            res_tx,
            res_rx,
            join_handle: Default::default(),
        }
    }

    /// If the worker is running, send a Stop message to it and wait until the
    /// underlying thread finishes.
    pub fn stop_and_wait(&self) {
        let handle = self.join_handle.take();
        if let Some(handle) = handle {
            self.req_tx
                .send(WorkerRequest::Stop)
                .expect("unable to send request");
            handle.join().expect("child thread panicked");
        }
    }

    /// Start the worker thread.
    pub fn start(&self) {
        let req_rx = self.req_rx.clone();
        let res_tx = self.res_tx.clone();

        let handle = thread::spawn(move || {
            loop {
                let req = req_rx.recv().unwrap();
                match req {
                    WorkerRequest::RunScript(req) => {
                        // TODO:
                        // * parse request
                        // * run script
                        // * send response
                        println!("{:?}", req.id);

                        res_tx
                            .send(WorkerResponse::Success { id: req.id })
                            .expect("could not send response");
                    }
                    WorkerRequest::Stop => {
                        break;
                    }
                }
            }
        });
        self.join_handle.set(Some(handle));
    }
}
