use std::any::Any;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use actors::{ActorPath, ActorRef, Props};
use actors::actor_cell::{ActorCell, SystemMessage};
use actors::actor_ref::eventual::Async;
use actors::cthulhu::Cthulhu;
use actors::name_resolver::NameResolver;
use actors::props::ActorFactory;
use actors::root_actor::RootActor;

/// This is failsafe used to relaunch consumer threads if they panic!.
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

/// The actor system is the struct that manages:
///
///   * The creation of the root actors.
///   * The consumer threads.
///   * Scheduling the actors.
///
/// It needs to be instantiated once at the beggining of the application. Then we need to specify
/// the number of consumer threads that will be allocated.
///
/// Calling `shutdown`, will drop all the actors and terminate the consumer threads.
/// Note that it will shut down the system even if some actors have still messages to handle.
pub struct ActorSystem {
    inner: Arc<InnerActorSystem>,
}

impl ActorSystem {
    /// Creates a new ActorSystem.
    ///
    /// Note that one thread is started.
    pub fn new(name: String) -> ActorSystem {
        let actor_system = ActorSystem { inner: Arc::new(InnerActorSystem::new(name)) };
        let cthulhu = Cthulhu::new(actor_system.clone());
        let cthulhu = ActorRef::with_cthulhu(cthulhu);
        *actor_system.inner.cthulhu.write().unwrap() = Some(cthulhu.clone());
        let user_actor_path = ActorPath::new_local("/user".to_owned());
        let user_actor_cell = ActorCell::new(Props::new(Arc::new(RootActor::new), ()),
                                                actor_system.clone(),
                                                cthulhu.clone(),
                                                user_actor_path.clone());
        let user_actor = ActorRef::with_cell(user_actor_cell, user_actor_path);
        user_actor.receive_system_message(SystemMessage::Start);
        *actor_system.inner.user_actor.write().unwrap() = Some(user_actor);
        let system_actor_path = ActorPath::new_local("/system".to_owned());
        let system_actor_cell = ActorCell::new(Props::new(Arc::new(RootActor::new), ()),
                                                actor_system.clone(),
                                                cthulhu.clone(),
                                                system_actor_path.clone());
        let system_actor = ActorRef::with_cell(system_actor_cell, system_actor_path);
        system_actor.receive_system_message(SystemMessage::Start);
        *actor_system.inner.system_actor.write().unwrap() = Some(system_actor);
        actor_system.spawn_threads(1);
        let name_resolver = actor_system.system_actor_of(Props::new(Arc::new(NameResolver::new), ()), "name_resolver".to_owned());
        *actor_system.inner.name_resolver.write().unwrap() = Some(name_resolver);
        actor_system
    }

    /// Spawns an Actor created using the Props given for the user.
    pub fn actor_of(&self, props: Arc<ActorFactory>, name: String) -> ActorRef {
        self.inner.actor_of(props, name)
    }

    /// Spawns an Actor created using the Props given for the system.
    pub fn system_actor_of(&self, props: Arc<ActorFactory>, name: String) -> ActorRef {
        self.inner.system_actor_of(props, name)
    }

    /// Shuts the actor system down.
    ///
    /// It will terminate all the actors (whether they still have messages to handle or not) and
    /// then terminate the consumer threads.
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Enqueues the given ActorRef in the queue of ActorRef with message to handle.
    pub fn enqueue_actor(&self, actor_ref: ActorRef) {
        self.inner.enqueue_actor(actor_ref);
    }

    /// Spawns a thread that will have ActorRef handle their messages.
    ///
    /// This thread can be terminated by calling `terminate_thread`.
    pub fn spawn_thread(&self) {
        let actors_queue = self.inner.actors_queue_receiver.clone();
        let rx = self.inner.consumer_threads_receiver.clone();
        let actor_system = self.clone();
        let _ = thread::spawn(move || {
            // This is a failsafe used to relaunch a consumer thread if it panic!
            let relauncher = Relauncher::new(actor_system.clone());
            loop {
                // We check if we received a termination request.
                match rx.lock().unwrap().try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        relauncher.cancel();
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                };

                // We try to get an ActorRef with a message to handle.
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

    /// Kills a consumer thread.
    pub fn terminate_thread(&self) {
        self.inner.terminate_thread();
    }

    /// Spawns n consumer threads.
    pub fn spawn_threads(&self, n: u32) {
        for _ in 0..n {
            self.spawn_thread();
        }
    }

    /// Kills n consumer threads.
    pub fn terminate_threads(&self, n: u32) {
        self.inner.terminate_threads(n);
    }

    /// Gives the ActorRef of the name resolver actor.
    pub fn name_resolver(&self) -> ActorRef {
        match self.inner.name_resolver.read().unwrap().as_ref() {
            None => panic!("The name resolver is not initialized."),
            Some(resolver) => resolver.clone(),
        }
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> ActorSystem {
        ActorSystem { inner: self.inner.clone() }
    }
}

struct InnerActorSystem {
    _name: String,
    // Communication channels to the co,sumer threads.
    consumer_threads_sender: Mutex<Sender<()>>,
    consumer_threads_receiver: Arc<Mutex<Receiver<()>>>,
    n_threads: Mutex<u32>,
    // Sends ActorRefs to be handled on that channel.
    actors_queue_sender: Mutex<Sender<ActorRef>>,
    // Receiving end to give to the thread pool.
    actors_queue_receiver: Arc<Mutex<Receiver<ActorRef>>>,
    cthulhu: RwLock<Option<ActorRef >>,
    user_actor: RwLock<Option<ActorRef>>,
    system_actor: RwLock<Option<ActorRef>>,
    // ActorRef to the name resolver.
    name_resolver: RwLock<Option<ActorRef>>,
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
            cthulhu: RwLock::new(None),
            user_actor: RwLock::new(None),
            system_actor: RwLock::new(None),
            name_resolver: RwLock::new(None),
        }
    }

    /// Spawns an Actor for the user with the given ActorFactory.
    ///
    /// This will be part of the user cator hierarchy.
    fn actor_of(&self, props: Arc<ActorFactory>, name: String) -> ActorRef {
        // Not having the user actor in a Mutex is ok because the actor_of function already has
        // mutual exclusion, so we are in the clear.
        match self.user_actor.read().unwrap().clone() {
            Some(user_actor) => {
                let future = user_actor.ask((props, name));
                let answer = future.await().unwrap();
                *Box::<Any>::downcast::<ActorRef>(answer).unwrap()
            },
            None => panic!("The user actor is not initialised"),
        }
    }

    fn system_actor_of(&self, props: Arc<ActorFactory>, name: String) -> ActorRef {
        // Not having the user actor in a Mutex is ok because the actor_of function already has
        // mutual exclusion, so we are in the clear.
        match self.system_actor.read().unwrap().clone() {
            Some(system_actor) => {
                let future = system_actor.ask((props, name));
                let answer = future.await().unwrap();
                *Box::<Any>::downcast::<ActorRef>(answer).unwrap()
            },
            None => panic!("The user actor is not initialised"),
        }
    }

    /// Shuts the actor system down.
    fn shutdown(&self) {
        // We have to get this out of the mutex, because terminate_threads would deadlock on
        // n_thread.
        let n = {*self.n_threads.lock().unwrap()};
        self.terminate_threads(n);
        *self.user_actor.write().unwrap() = None;
        *self.system_actor.write().unwrap() = None;
        *self.cthulhu.write().unwrap() = None;
    }

    /// Enqueues the given ActorRef in the list of ActorRef with messages to be handled.
    fn enqueue_actor(&self, actor_ref: ActorRef) {
        match self.actors_queue_sender.lock().unwrap().send(actor_ref) {
            Ok(_) => return,
            Err(_) => {
                self.shutdown();
                panic!("The communication channel for messages is disconnected, this is bad!");
            }
        }
    }

    /// Kills a consumer thread.
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
    fn drop(&mut self) { }
}
