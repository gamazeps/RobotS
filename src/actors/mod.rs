use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::thread;

/// Emum used to xrap messages.
///   * If the message is normal data it is in a Data variant.
///   * If the message is a Command it has its own variant.
pub enum Message {
    /// Variant used to pass real data around (done by passing a Box to it).
    Data(Box<Any + Send>),
    /// Dummy command.
    Command,
}

/// Utility type to wrap `Actor`s in a thread safe manner.
pub type ActorRef = Arc<Mutex<Actor>>;

/// Trait used for actors, implementing this trait is enough to be an Actor.
pub trait Actor: Send {
    /// Gets the `Actor`'s `ActorSystem`.
    fn actor_system(&self) -> Arc<ActorSystem>;
    /// Method to call on an `Actor` for him to put a message in his message queue.
    fn receive(&self, Message);
    /// Method to call on an `Actor` for him to handle a message from his message queue.
    fn handle_message(&self);
    /// Sends a `Message` to the given `ActorRef`
    fn send_message(&self, actor_ref: ActorRef, message: Message) {
        self.actor_system().send_to_actor(actor_ref, message);
    }
    /// DEV ONLY: Sends a message to the first ActorRef in known_actors.
    // Used on dev, to be removed afterwards.
    fn send_to_first(&self, message: Message);
}

/// This is a very basic `Actor` which can deal with `String` messages by printing them,
/// any other type of `Data` will have the `Printer` do nothing.
pub struct Printer {
    name: Arc<String>,
    message_queue: Arc<Mutex<VecDeque<Message>>>,
    actor_system: Arc<ActorSystem>,
    known_actors: Arc<Mutex<Vec<ActorRef>>>,
}

impl Printer {
    fn new(name: String, actor_system: Arc<ActorSystem>, known_actors: Vec<ActorRef>) -> Printer {
        Printer {
            name: Arc::new(name),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            actor_system: actor_system,
            known_actors: Arc::new(Mutex::new(known_actors)),
        }
    }
}

impl Actor for Printer {
    fn actor_system(&self) -> Arc<ActorSystem> {
        self.actor_system.clone()
    }

    fn receive(&self, message: Message) {
        self.message_queue.lock().unwrap().push_back(message);
    }

    fn handle_message(&self) {
        let message = self.message_queue.lock().unwrap().pop_front().unwrap();

        println!("({}) treats a message", self.name);
        match message {
            Message::Data(ref data) => {
                match data.downcast_ref::<String>() {
                    Some(s) => println!("Received data: ({})", s),
                    None => println!("Message is dropped"),
                }
            },
            Message::Command => println!("Receiced a command"),
        }
    }


    fn send_to_first(&self, message: Message) {
        let actor_ref = self.known_actors.lock().unwrap()[0].clone();
        self.send_message(actor_ref, message);
    }
}

/// Wrapper around the threads handle and termination sender.
type ConsumerThread = (thread::JoinHandle<()>, Sender<()>);

/// A basic actor system, that handles the creation, distribution and handling of messages.
/// `Actor`s are simply put in a FIFO when they receive a message, they are poped from it when
/// `handle_actor_message` is called.
/// `ActorSystem::spawn_thread` allows to create simple threads that will handle `Actor`s messages.
pub struct ActorSystem {
    // TODO(gamazeps): Use an unordered container instead.
    // There is currently an issue with having an ActorRef as a Arc<Mutex<Actor + Eq + Hash>>.
    actors_table: Arc<Mutex<Vec<ActorRef>>>,
    actors_queue: Arc<Mutex<VecDeque<ActorRef>>>,
    consumer_threads: Arc<Mutex<Vec<ConsumerThread>>>,
    myself: Arc<Mutex<Option<Weak<ActorSystem>>>>,
}

unsafe impl Send for ActorSystem {}

impl ActorSystem {
    /// Creates a new `ActorSystem` and gives an `Arc` to it.
    pub fn new() -> Arc<ActorSystem> {
        let actor_system = Arc::new(ActorSystem {
            actors_table: Arc::new(Mutex::new(Vec::new())),
            actors_queue: Arc::new(Mutex::new(VecDeque::new())),
            consumer_threads: Arc::new(Mutex::new(Vec::new())),
            myself: Arc::new(Mutex::new(None)),
        });
        ActorSystem::init(actor_system.clone());
        actor_system
    }

    fn init(me: Arc<ActorSystem>) {
        *me.myself.lock().unwrap() = Some(Arc::downgrade(&me));
    }


    fn myself(&self) -> Option<Weak<ActorSystem>> {
        self.myself.lock().unwrap().clone()
    }

    /// Creates an `Actor` (currently just a `Printer`), adds it to the `actors_table` and gives an
    /// `ActorRef` to it.
    pub fn spawn_actor(&self, name: String, known_actors: Vec<ActorRef>) -> ActorRef {
        let actor_ref = Arc::new(Mutex::new(Printer::new(
                    name, self.myself().unwrap().upgrade().unwrap(), known_actors)));
        {
            let mut actors_table = self.actors_table.lock().unwrap();
            actors_table.push(actor_ref.clone());
        }
        actor_ref
    }

    /// Sends `message` to the actor corresponding to `actor_ref`.
    pub fn send_to_actor(&self, actor_ref: ActorRef, message: Message) {
        {
            let actor = actor_ref.lock().unwrap();
            actor.receive(message);
        }
        {
            let mut actors_queue = self.actors_queue.lock().unwrap();
            actors_queue.push_back(actor_ref);
        }
    }

    /// Method to call on an `ActorSystem` to have him pop an `ActorRef` from his `actors_queue`
    /// and handle one of his messages.
    pub fn handle_actor_message(&self) {
        let actor_ref = {self.actors_queue.lock().unwrap().pop_front()};
        if let Some(actor) = actor_ref {
            actor.lock().unwrap().handle_message();
        }
    }

    /// Spawns a thread that will consume messages from the `ActorRef` in `actors_queue`.
    /// This thread can be terminated by calling `terminate_thread`.
    pub fn spawn_thread(actor_system: Arc<ActorSystem>) {
        let (tx, rx) = channel();
        let thread_actor_system =  actor_system.clone();
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
                thread_actor_system.handle_actor_message();
            }
        });
        actor_system.add_thread(handle, tx);
    }

    fn add_thread(&self, handle: thread::JoinHandle<()>, sender: Sender<()>) {
        self.consumer_threads.lock().unwrap().push((handle, sender));
    }

    /// Kills a consumer thread of the `ActorSystem`.
    pub fn terminate_thread(&self) {
        let (handle, tx) = {self.consumer_threads.lock().unwrap().pop().unwrap()};
        let _res = tx.send(());
        let _res = handle.join();
    }
}
