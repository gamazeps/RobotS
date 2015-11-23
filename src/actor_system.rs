use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::thread;

use {Actor, ActorRef, CanReceive, Props};
use actor_cell::ActorCell;

/// Wrapper around the threads handle and termination sender.
type ConsumerThread = (thread::JoinHandle<()>, Sender<()>);

pub struct ActorSystem {
    name: Arc<String>,
    // For now we will have the worker pool in the system.
    // TODO(find a way to have a clean way to separate system and user threads).
    consumer_threads: Arc<Mutex<Vec<ConsumerThread>>>,
    actors_queue: Arc<Mutex<VecDeque<Arc<CanReceive + Sync>>>>,
}

impl ActorSystem {
    pub fn new(name: String) -> ActorSystem {
        ActorSystem {
            name: Arc::new(name),
            consumer_threads: Arc::new(Mutex::new(Vec::new())),
            actors_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn actor_of<Args: Copy + Sync + Send, A: Actor>(&self, props: Props<Args, A>) -> ActorRef<Args, A> {
        let actor = props.create();
        let actor_cell = ActorCell::new(actor, props, self.clone());
        ActorRef::with_cell(actor_cell)
    }

    /// Spawns a thread that will consume messages from the `ActorRef` in `actors_queue`.
    /// This thread can be terminated by calling `terminate_thread`.
    pub fn spawn_thread(&self) {
        let (tx, rx) = channel();
        let thread_system = self.clone();
        let handle = thread::spawn(move || {
            loop {
                // If we received a () we kill the thread.
                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        println!("Terminating a consumer thread.");
                        break;
                    },
                    Err(TryRecvError::Empty) => {}
                };
                // Else we try to prcess a message.
                let actor_ref = {thread_system.actors_queue.lock().unwrap().pop_front()};
                match actor_ref {
                    Some(actor) => actor.handle(),
                    None => {
                        println!("No waiting actor");
                        thread::sleep_ms(500);
                    }
                }
            }
        });
        self.add_thread(handle, tx);
    }

    fn add_thread(&self, handle: thread::JoinHandle<()>, sender: Sender<()>) {
        self.consumer_threads.lock().unwrap().push((handle, sender));
    }

    /// Kills a consumer thread of the `ActorSystem`.
    pub fn terminate_thread(&self) {
        self.terminate_threads(1);
    }

    /// Spawns n threads that will consume messages from the `ActorRef` in `actors_queue`.
    pub fn spawn_threads(&self, n: u32) {
        for _ in 0..n {
            self.spawn_thread();
        }
    }

    /// Kills n consumer threads.
    pub fn terminate_threads(&self, n: u32) {
        let mut handles = Vec::new();
        for _ in 0..n {
            let (handle, tx) = {self.consumer_threads.lock().unwrap().pop().unwrap()};
            let _res = tx.send(());
            handles.push(handle);
        }
        for handle in handles {
            let _res = handle.join();
        }
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> ActorSystem {
        ActorSystem {
            name: self.name.clone(),
            consumer_threads: self.consumer_threads.clone(),
            actors_queue: self.actors_queue.clone(),
        }
    }
}
