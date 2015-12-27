use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use actors::{Actor, ActorRef, Arguments, CanReceive, Props};
use actors::cthulhu::Cthulhu;
use actors::user_actor::UserActorRef;

struct Relauncher {
    actor_system: ActorSystem,
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
    inner: Arc<InnerActorSystem>,
}

impl ActorSystem {
    /// Creates a new ActorSystem.
    ///
    /// Note that no threads are started.
    pub fn new(name: String) -> ActorSystem {
        let actor_system = ActorSystem { inner: Arc::new(InnerActorSystem::new(name)) };
        let cthulhu = Arc::new(Cthulhu::new(actor_system.clone()));
        *actor_system.inner.cthulhu.lock().unwrap() = Some(cthulhu.clone());
        let user_actor = UserActorRef::new(actor_system.clone(), cthulhu.clone());
        *actor_system.inner.user_actor.lock().unwrap() = Some(user_actor);
        actor_system
    }

    /// Spawns an Actor of type A, created using the Props given.
    pub fn actor_of<Args: Arguments, A: Actor + 'static>(&self,
                                                         props: Props<Args, A>,
                                                         name: String)
                                                         -> Arc<ActorRef<Args, A>> {
        self.inner.actor_of(props, name)
    }

    /// Shuts the actor system down.
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Enqueues the given Actor on the queue of Actors with something to handle.
    pub fn enqueue_actor<Args, A>(&self, actor_ref: Arc<ActorRef<Args, A>>)
        where Args: Arguments,
              A: Actor + 'static
    {
        self.inner.enqueue_actor(actor_ref);
    }

    /// Spawns a thread that will consume messages from the `ActorRef` in `actors_queue`.
    /// This thread can be terminated by calling `terminate_thread`.
    pub fn spawn_thread(&self) {
        let actors_queue = self.inner.actors_queue_receiver.clone();
        let rx = self.inner.consumer_threads_receiver.clone();
        let actor_system = self.clone();
        let _ = thread::spawn(move || {
            let relauncher = Relauncher::new(actor_system.clone());
            // Here we beed to give it an initial value, so Cthulhu it is.
            // Keep on tryieng relaunching threads as they fail.
            loop {
                // If we received a () we kill the thread.
                match rx.lock().unwrap().try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        // println!("Terminating a consumer thread.");
                        relauncher.cancel();
                        break;
                    }
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
                        relauncher.cancel();
                        actor_system.shutdown();
                        panic!("The actors queue failed, something is very wrong");
                    }
                }
            }
        });
        *self.inner.n_threads.lock().unwrap() += 1;
    }

    /// Kills a consumer thread of the `ActorSystem`.
    pub fn terminate_thread(&self) {
        self.inner.terminate_thread();
    }

    /// Spawns n threads that will consume messages from the `ActorRef` in `actors_queue`.
    pub fn spawn_threads(&self, n: u32) {
        for _ in 0..n {
            self.spawn_thread();
        }
    }

    /// Kills n consumer threads.
    pub fn terminate_threads(&self, n: u32) {
        self.inner.terminate_threads(n);
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> ActorSystem {
        ActorSystem { inner: self.inner.clone() }
    }
}

struct InnerActorSystem {
    _name: String,
    // For now we will have the worker pool in the system.
    // FIXME(gamazeps): find a way to have a clean way to separate system and user threads.
    consumer_threads_sender: Mutex<Sender<()>>,
    consumer_threads_receiver: Arc<Mutex<Receiver<()>>>,
    n_threads: Mutex<u32>,
    // Sends Canreceive to be handled on that channel.
    actors_queue_sender: Mutex<Sender<Arc<CanReceive>>>,
    // Receiving end to give to the thread pool.
    actors_queue_receiver: Arc<Mutex<Receiver<Arc<CanReceive>>>>,
    cthulhu: Mutex<Option<Arc<Cthulhu>>>,
    user_actor: Mutex<Option<UserActorRef>>,
}

impl InnerActorSystem {
    fn new(name: String) -> InnerActorSystem {
        let (tx_queue, rx_queue) = channel();
        let (tx_thread, rx_thread) = channel();
        InnerActorSystem {
            _name: name,
            consumer_threads_sender: Mutex::new(tx_thread),
            consumer_threads_receiver: Arc::new(Mutex::new(rx_thread)),
            n_threads: Mutex::new(0u32),
            actors_queue_sender: Mutex::new(tx_queue),
            actors_queue_receiver: Arc::new(Mutex::new(rx_queue)),
            cthulhu: Mutex::new(None),
            user_actor: Mutex::new(None),
        }
    }

    /// Spawns an Actor of type A, created using the Props given.
    fn actor_of<Args: Arguments, A: Actor + 'static>(&self,
                                                     props: Props<Args, A>,
                                                     name: String)
                                                     -> Arc<ActorRef<Args, A>> {
        // Not having the user actor in a Mutex in ok because the actor_of function already has
        // mutual exclusion, so we are in the clear.
        match self.user_actor.lock().unwrap().clone() {
            Some(user_actor) => user_actor.actor_of(props, name),
            None => panic!("The user actor is not initialised"),
        }
    }

    /// Shuts the actor system down.
    fn shutdown(&self) {
        // We have to get this out of the mutex, because terminate_threads would deadlock on
        // n_thread.
        let n = {
            *self.n_threads.lock().unwrap()
        };
        self.terminate_threads(n);
        *self.user_actor.lock().unwrap() = None;
        *self.cthulhu.lock().unwrap() = None;
    }

    /// Enqueues the given Actor on the queue of Actors with something to handle.
    fn enqueue_actor<Args, A>(&self, actor_ref: Arc<ActorRef<Args, A>>)
        where Args: Arguments,
              A: Actor + 'static
    {
        match self.actors_queue_sender.lock().unwrap().send(actor_ref) {
            Ok(_) => return,
            Err(_) => {
                self.shutdown();
                panic!("The communication channel for messages is disconnected, this is bad!");
            }
        }
    }

    /// Kills a consumer thread of the `ActorSystem`.
    fn terminate_thread(&self) {
        let _ = self.consumer_threads_sender.lock().unwrap().send(());
        *self.n_threads.lock().unwrap() -= 1;
    }

    /// Kills n consumer threads.
    fn terminate_threads(&self, n: u32) {
        for _ in 0..n {
            self.terminate_thread();
        }
    }
}

impl Drop for InnerActorSystem {
    fn drop(&mut self) {
        // println!("Dropping the {} actor system.", self.name);
    }
}
