use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};

use super::{Message, ActorRef, Actor, ActorSystem};

/// This is a very basic `Actor` which can deal with `String` messages by printing them,
/// any other type of `Data` will have the `Printer` do nothing.
pub struct Printer {
    name: Arc<String>,
    message_queue: Arc<Mutex<VecDeque<Message>>>,
    actor_system: Arc<ActorSystem>,
    known_actors: Arc<Mutex<Vec<ActorRef>>>,
    myself: Arc<Mutex<Option<Weak<Mutex<Actor>>>>>,
}

impl Actor for Printer {
    fn new(name: String, actor_system: Arc<ActorSystem>, known_actors: Vec<ActorRef>) -> ActorRef {
        let actor_ref = Arc::new(Mutex::new(Printer {
            name: Arc::new(name),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            actor_system: actor_system,
            known_actors: Arc::new(Mutex::new(known_actors)),
            myself: Arc::new(Mutex::new(None)),
        }));
        Printer::init(actor_ref.clone());
        actor_ref
    }

    fn init(me: ActorRef) {
        // There are 2 temp values to satisfy the borrow checker.
        let x = me.lock().unwrap().myself();
        let mut y = x.lock().unwrap();
        *y = Some(Arc::downgrade(&me));
    }

    fn actor_ref(&self) -> ActorRef {
        let x = self.myself();
        let y = x.lock().unwrap(); // Get the Option<Weak> out of the Mutex.
        let z = match *y {
            Some(ref w) => w,
            None => panic!(""),
        }; // Get the Weak out of the Option.
        z.upgrade().unwrap()
    }

    fn myself(&self) -> Arc<Mutex<Option<Weak<Mutex<Actor>>>>> {
        self.myself.clone()
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

    fn actor_system(&self) -> Arc<ActorSystem> { self.actor_system.clone() }
    fn receive(&self, message: Message) { self.message_queue.lock().unwrap().push_back(message); }
    fn send_to_first(&self, message: Message) {
        let actor_ref = self.known_actors.lock().unwrap()[0].clone();
        self.send_message(actor_ref, message);
    }
}

/// This is a very basic `Actor` which can deal with `u32` messages by counting up to them,
/// any other type of `Data` will have the `Counter` do nothing.
pub struct Counter {
    name: Arc<String>,
    message_queue: Arc<Mutex<VecDeque<Message>>>,
    actor_system: Arc<ActorSystem>,
    known_actors: Arc<Mutex<Vec<ActorRef>>>,
    myself: Arc<Mutex<Option<Weak<Mutex<Actor>>>>>,
}

impl Actor for Counter {
    fn new(name: String, actor_system: Arc<ActorSystem>, known_actors: Vec<ActorRef>) -> ActorRef {
        let actor_ref = Arc::new(Mutex::new(Counter {
            name: Arc::new(name),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            actor_system: actor_system,
            known_actors: Arc::new(Mutex::new(known_actors)),
            myself: Arc::new(Mutex::new(None)),
        }));
        Counter::init(actor_ref.clone());
        actor_ref
    }

    fn init(me: ActorRef) {
        // There are 2 temp values to satisfy the borrow checker.
        let x = me.lock().unwrap().myself();
        let mut y = x.lock().unwrap();
        *y = Some(Arc::downgrade(&me));
    }

    fn actor_ref(&self) -> ActorRef {
        let x = self.myself();
        let y = x.lock().unwrap(); // Get the Option<Weak> out of the Mutex.
        let z = match *y {
            Some(ref w) => w,
            None => panic!(""),
        }; // Get the Weak out of the Option.
        z.upgrade().unwrap()
    }

    fn myself(&self) -> Arc<Mutex<Option<Weak<Mutex<Actor>>>>> {
        self.myself.clone()
    }

    fn handle_message(&self) {
        let message = self.message_queue.lock().unwrap().pop_front().unwrap();

        println!("({}) treats a message", self.name);
        match message {
            Message::Data(ref data) => {
                match data.downcast_ref::<u32>() {
                    Some(n) => {
                        println!("Received data: ({})", n);
                        for i in 0..*n {
                            println!("{}", i);
                        }
                    },
                    None => println!("Message is dropped"),
                }
            },
            Message::Command => println!("Receiced a command"),
        }
    }

    fn actor_system(&self) -> Arc<ActorSystem> { self.actor_system.clone() }
    fn receive(&self, message: Message) { self.message_queue.lock().unwrap().push_back(message); }
    fn send_to_first(&self, message: Message) {
        let actor_ref = self.known_actors.lock().unwrap()[0].clone();
        self.send_message(actor_ref, message);
    }
}
