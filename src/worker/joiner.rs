use super::Error;
use crossbeam::channel::Sender;
use threadpool::ThreadPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct Joiner {
    notify_sender: Sender<Result<Uuid, Error>>,
    thread_pool: ThreadPool,
}

impl Joiner {
    pub fn new(thread_count: usize, sender: Sender<Result<Uuid, Error>>) -> Self {
        Joiner {
            thread_pool: ThreadPool::new(thread_count),
            notify_sender: sender,
        }
    }

    pub fn queue_job(&self, wait_result: Result<Uuid, Error>) {
        let sender = self.notify_sender.clone();
        self.thread_pool.execute(move || {
            if let Err(e) = sender.send(wait_result) {
                panic!("failed to send job error: {:?}", e)
            }
        })
    }
}

impl Drop for Joiner {
    fn drop(&mut self) {
        self.thread_pool.join();
    }
}
