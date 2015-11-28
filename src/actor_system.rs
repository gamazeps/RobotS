use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::thread;

use {Actor, ActorRef, CanReceive, Props};
use cthulhu::Cthulhu;
use user_actor::UserActorRef;

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
    // TODO(gamazeps): Have a CanHandle Trait for that.
    actors_queue: Arc<Mutex<VecDeque<Arc<CanReceive>>>>,
    cthulhu: Arc<Cthulhu>,
    user_actor: Mutex<Option<UserActorRef>>,
}

impl ActorSystem {
    /// Creates a new ActorSystem.
    ///
    /// Note that no threads are started.
    pub fn new(name: String) -> ActorSystem {
        let actor_system = ActorSystem {
            name: Arc::new(name),
            consumer_threads: Arc::new(Mutex::new(Vec::new())),
            actors_queue: Arc::new(Mutex::new(VecDeque::new())),
            cthulhu: Arc::new(Cthulhu::new()),
            user_actor: Mutex::new(None),
        };
        actor_system.spawn_user_actor();
        actor_system
    }

    fn spawn_user_actor(&self) {
        let user_actor = UserActorRef::new(self.clone(), self.cthulhu.clone());
        *self.user_actor.lock().unwrap() = Some(user_actor);
    }

    /// Spawns an Actor of type A, created using the Props given.
    pub fn actor_of<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static>(&self, props: Props<Args, M, A>) -> ActorRef<Args, M, A> {
        let user_actor = self.user_actor.lock().unwrap().clone();
        match user_actor {
            Some(user_actor) => user_actor.actor_of(props),
            None => panic!("The user actor is not initialised"),
        }
    }

    /// Enqueues the given Actor on the queue of Actors with something to handle.
    pub fn enqueue_actor<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static>(&self, actor_ref: ActorRef<Args, M, A>) {
        self.actors_queue.lock().unwrap().push_back(Arc::new(actor_ref));
    }

    /// Spawns a thread that will consume messages from the `ActorRef` in `actors_queue`.
    /// This thread can be terminated by calling `terminate_thread`.
    pub fn spawn_thread(&self) {
        let (tx, rx) = channel();
        let thread_system = self.clone();
        // When we have the two threads, use one to monitoring the other one (as thought before).
        // In order to know which actor failed, use a channel to transmit the CanReceive that
        // caused the failure.
        // Last correct returned value by the receiver wil be the actor that panicked.
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
                for actor in actor_ref.iter() {
                    actor.handle();
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
            cthulhu: self.cthulhu.clone(),
            user_actor: Mutex::new(self.user_actor.lock().unwrap().clone()),
        }
    }
}
