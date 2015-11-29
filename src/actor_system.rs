use std::any::Any;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use {Actor, ActorRef, CanReceive, Props};
use actor_cell::ActorCell;

/// Wrapper around the threads handle and termination sender.
type ConsumerThread = (thread::JoinHandle<()>, Sender<()>);

/// ActorSystem, the struct that manages the creation of everything and that everything does what
/// it is supposed to do.
///
/// NOTE: It currently holds the consumer threads and do not create the user, system and root
/// actors.
pub struct ActorSystem {
    name: Arc<String>,
    // For now we will have the worker pool in the system.
    // TODO(find a way to have a clean way to separate system and user threads).
    consumer_threads: Arc<Mutex<Vec<ConsumerThread>>>,
    // Sends Canreceive to be handled on that channel.
    actors_queue_sender: Arc<Mutex<Sender<Arc<CanReceive>>>>,
    // Receiving end to give to the thread pool.
    actors_queue_receiver: Arc<Mutex<Receiver<Arc<CanReceive>>>>,
}

impl ActorSystem {
    /// Creates a new ActorSystem.
    ///
    /// Note that no threads are started.
    pub fn new(name: String) -> ActorSystem {
        let (tx, rx) = channel();
        ActorSystem {
            name: Arc::new(name),
            consumer_threads: Arc::new(Mutex::new(Vec::new())),
            actors_queue_sender: Arc::new(Mutex::new(tx)),
            actors_queue_receiver: Arc::new(Mutex::new(rx)),
        }
    }

    /// Spawns an Actor of type A, created using the Props given.
    pub fn actor_of<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static>(&self, props: Props<Args, M, A>) -> ActorRef<Args, M, A> {
        let actor = props.create();
        let actor_cell = ActorCell::new(actor, props, self.clone());
        ActorRef::with_cell(actor_cell)
    }

    /// Enqueues the given Actor on the queue of Actors with something to handle.
    pub fn enqueue_actor<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static>(&self, actor_ref: ActorRef<Args, M, A>) {
        match self.actors_queue_sender.lock().unwrap().send(Arc::new(actor_ref)) {
            Ok(_) => return,
            Err(_) => {
                println!("The communication channel for messages is disconnected, this is bad!");
                exit(1);
            },
        }
    }

    /// Spawns a thread that will consume messages from the `ActorRef` in `actors_queue`.
    /// This thread can be terminated by calling `terminate_thread`.
    pub fn spawn_thread(&self) {
        let (tx, rx) = channel();
        let actors_queue = self.actors_queue_receiver.clone();
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

                // Else we try to process a message.
                let actor_ref = {
                    let lock = actors_queue.lock().unwrap();
                    lock.try_recv()
                };

                match actor_ref {
                    Ok(actor_ref) => actor_ref.handle(),
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => {
                        println!("The actors queue failed, something is very wrong");
                        exit(1);
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
            actors_queue_sender: self.actors_queue_sender.clone(),
            actors_queue_receiver: self.actors_queue_receiver.clone(),
        }
    }
}
