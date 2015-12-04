use std::process::exit;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use actors::{Actor, ActorRef, CanReceive, Message, Props};
use actors::cthulhu::Cthulhu;
use actors::user_actor::UserActorRef;

struct Relauncher {
    actor_system:  ActorSystem,
    active: bool,
}

impl Relauncher {
    fn new(actor_system: ActorSystem) -> Relauncher {
        Relauncher {
            actor_system: actor_system,
            active: true,
        }
    }

    fn cancel(mut self) {
        self.active = false;
    }
}

impl Drop for Relauncher {
    fn drop(&mut self) {
        if self.active {
            self.actor_system.spawn_thread();
        }
    }
}

/// ActorSystem, the struct that manages the creation of everything and that everything does what
/// it is supposed to do.
pub struct ActorSystem {
    name: Arc<String>,
    // For now we will have the worker pool in the system.
    // TODO(find a way to have a clean way to separate system and user threads).
    consumer_threads_sender: Arc<Mutex<Sender<()>>>,
    consumer_threads_receiver: Arc<Mutex<Receiver<()>>>,
    // Sends Canreceive to be handled on that channel.
    actors_queue_sender: Arc<Mutex<Sender<Arc<CanReceive>>>>,
    // Receiving end to give to the thread pool.
    actors_queue_receiver: Arc<Mutex<Receiver<Arc<CanReceive>>>>,
    cthulhu: Arc<Cthulhu>,
    user_actor: Mutex<Option<UserActorRef>>,
}

impl ActorSystem {
    /// Creates a new ActorSystem.
    ///
    /// Note that no threads are started.
    pub fn new(name: String) -> ActorSystem {
        let (tx_queue, rx_queue) = channel();
        let (tx_thread, rx_thread) = channel();
        let actor_system = ActorSystem {
            name: Arc::new(name),
            consumer_threads_sender: Arc::new(Mutex::new(tx_thread)),
            consumer_threads_receiver: Arc::new(Mutex::new(rx_thread)),
            actors_queue_sender: Arc::new(Mutex::new(tx_queue)),
            actors_queue_receiver: Arc::new(Mutex::new(rx_queue)),
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
    pub fn actor_of<Args: Message, M: Message, A: Actor<M> + 'static>(&self, props: Props<Args, M, A>) -> Arc<ActorRef<Args, M, A>> {
        let user_actor = self.user_actor.lock().unwrap().clone();
        match user_actor {
            Some(user_actor) => user_actor.actor_of(props),
            None => panic!("The user actor is not initialised"),
        }
    }

    /// Enqueues the given Actor on the queue of Actors with something to handle.
    pub fn enqueue_actor<Args: Message, M: Message, A: Actor<M> + 'static>(&self, actor_ref: Arc<ActorRef<Args, M, A>>) {
        match self.actors_queue_sender.lock().unwrap().send(actor_ref) {
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
        let actors_queue = self.actors_queue_receiver.clone();
        let rx = self.consumer_threads_receiver.clone();
        let actor_system =  self.clone();
        let _ = thread::spawn(move || {
            let relauncher = Relauncher::new(actor_system);
            // Here we beed to give it an initial value, so Cthulhu it is.
            // Keep on tryieng relaunching threads as they fail.
            loop {
                // If we received a () we kill the thread.
                match rx.lock().unwrap().try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        println!("Terminating a consumer thread.");
                        relauncher.cancel();
                        break;
                    },
                    Err(TryRecvError::Empty) => {}
                };

                // Else we try to prcess a message.
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
    }

    /// Kills a consumer thread of the `ActorSystem`.
    pub fn terminate_thread(&self) {
        let _ = self.consumer_threads_sender.lock().unwrap().send(());
    }

    /// Spawns n threads that will consume messages from the `ActorRef` in `actors_queue`.
    pub fn spawn_threads(&self, n: u32) {
        for _ in 0..n {
            self.spawn_thread();
        }
    }

    /// Kills n consumer threads.
    pub fn terminate_threads(&self, n: u32) {
        for _ in 0..n {
            self.terminate_thread();
        }
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> ActorSystem {
        ActorSystem {
            name: self.name.clone(),
            consumer_threads_sender: self.consumer_threads_sender.clone(),
            consumer_threads_receiver: self.consumer_threads_receiver.clone(),
            actors_queue_sender: self.actors_queue_sender.clone(),
            actors_queue_receiver: self.actors_queue_receiver.clone(),
            cthulhu: self.cthulhu.clone(),
            user_actor: Mutex::new(self.user_actor.lock().unwrap().clone()),
        }
    }
}
