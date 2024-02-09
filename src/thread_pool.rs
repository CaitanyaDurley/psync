use std::{sync::{mpsc, Arc, Mutex}, thread};

type Job = Box<dyn FnOnce() + Send>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    queue: Option<mpsc::Sender<Job>>,
}

struct Worker {
    #[allow(dead_code)]
    id: usize,
    handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let handle = thread::spawn(move || {
            loop {
                let rx = receiver.lock().unwrap();
                match rx.recv()  {
                    Ok(job) => job(),
                    Err(_) => break,
                };
            }
        });
        Self {
            id,
            handle: Some(handle),
        }
    }
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        assert!(num_threads > 0);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(num_threads);
        for id in 0..num_threads {
            let receiver = Arc::clone(&receiver);
            workers.push(Worker::new(id, receiver));
        }
        Self {
            workers,
            queue: Some(sender),
        }
    }

    pub fn run<F: FnOnce() + Send + 'static>(&self, f: F) {
        let job = Box::new(f);
        // self.queue.send(job).unwrap();
        self.queue.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.queue.take());
        for worker in self.workers.iter_mut() {
            if let Some(h) = worker.handle.take() {
                h.join().unwrap();
            };
        }
    }
}
