// src/main
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};
use std::sync::Condvar;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;
type Result<T> = anyhow::Result<T>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute_as_future<T, F>(&self, f: F) -> Future<T>
        where F: FnOnce() -> Result<T> + Send + 'static,
              T: Send + 'static
    {
        let mutex_cond: Arc<(Mutex<Option<Result<T>>>, Condvar)> = Arc::new((Mutex::new(None), Condvar::new()));
        let future_clone = Arc::clone(&mutex_cond);
        let thread_clone = Arc::clone(&mutex_cond);

        let future = Future::new(future_clone);

        let f = move || {
            let result = f();
            let mut data = thread_clone.0.lock().unwrap();
            data.replace(result);
            thread_clone.1.notify_all();
        };
        self.execute(f);

        future
    }

    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }

    pub fn execute_all_and_await<F>(&self, fs: Vec<F>) where
        F: FnOnce() + Send + 'static
    {
        let cd = Arc::new(CountDownLatch::new(fs.len()));
        for f in fs {
            let cd_clone = Arc::clone(&cd);
            self.execute(move || {
                f();
                cd_clone.count_down()
            })
        }
        cd.await_complete()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => job(),
                Err(_) => break
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

pub(crate) struct CountDownLatch {
    count: usize,
    condvar: Arc<(Mutex<usize>, Condvar)>,
}

impl CountDownLatch {
    pub(crate) fn new(count: usize) -> Self {
        let mutex = Mutex::new(count);
        let condvar = Condvar::new();
        CountDownLatch {
            count,
            condvar: Arc::new((mutex, condvar)),
        }
    }

    pub(crate) fn count_down(&self) {
        let (mutex, condvar) = &*self.condvar;
        let mut count = mutex.lock().unwrap();
        if *count > 0 {
            *count -= 1;
            if *count == 0 {
                // If count reaches zero, notify all waiting threads
                condvar.notify_all();
            }
        }
    }

    pub(crate) fn await_complete(&self) {
        let (mutex, condvar) = &*self.condvar;
        let mut count = mutex.lock().unwrap();
        while *count > 0 {
            count = condvar.wait(count).unwrap();
        }
    }
}

pub(crate) struct Future<T> {
    condvar: Arc<(Mutex<Option<Result<T>>>, Condvar)>,
    is_done: bool
}

impl<T> Future<T> {
    fn new(condvar: Arc<(Mutex<Option<Result<T>>>, Condvar)>) -> Future<T> {
        Future {
            condvar,
            is_done: false
        }
    }

    pub(crate) fn is_done(&self) -> bool{
        self.is_done
    }

    pub(crate) fn try_get(&mut self) -> Option<Result<T>> {
        let (mutex, _) = &*self.condvar;
        let mut data = mutex.lock().unwrap();
        match data.take() {
            None => {None}
            Some(data) => {
                self.is_done = true;
                Some(data)}
        }
    }

    pub(crate) fn get(& self) -> Result<T> {
        let (mutex, condvar) = &*self.condvar;
        let mut data = mutex.lock().unwrap();
        while let None = *data {
            data = condvar.wait(data).unwrap();
        }
        data.take().unwrap()
    }
}
