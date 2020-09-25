use std::thread::JoinHandle;
use std::thread;

use std::sync::mpsc::{Sender, self};
use std::sync::{Arc, Mutex};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub enum WorkerMessage {
    START,
    JOB(Job),
    STOP
}

pub struct ThreadPool {
    name: String,
    sender: Sender<WorkerMessage>,
    workers: Vec<JoinHandle<()>>
}

// create custom worker instead of raw handles in pool!!

impl ThreadPool {

    pub fn submit(&mut self, job: impl FnOnce() + Send + 'static) {
        self.sender.send(WorkerMessage::JOB(Box::new(job)));
    }

    pub fn stop(self) {
        for _i in 0..self.workers.len() {
            self.sender.send(WorkerMessage::STOP).unwrap();
        }

        self.workers.into_iter()
            .for_each(|handle| handle.join().unwrap());
    }

    pub fn new(name: String, num_workers: u8) -> ThreadPool {
        let mut workers = vec![];

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for i in 0..num_workers {
            let worker_receiver = receiver.clone();
            let worker_name = format!("{}_{}", &name, i);

            workers.push(thread::spawn(move || {
                loop {
                    let guard = worker_receiver.lock().unwrap();
                    match guard.recv().unwrap() {
                        WorkerMessage::START => println!("started worker: {}", &worker_name),
                        WorkerMessage::JOB(job) => {
                            let time = std::time::SystemTime::now();
                            println!("running job for worker: {}", &worker_name);
                            job();
                            println!("completed job: {} in: {:?}", &worker_name, time.elapsed().unwrap());
                        },
                        WorkerMessage::STOP => {
                            println!("terminating worker: {}", &worker_name);
                            break;
                        }
                    }
                }
            }));
        }

        ThreadPool {
            name,
            sender,
            workers
        }
    }
}

