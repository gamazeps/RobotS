#![feature(plugin)]
#![plugin(clippy)]

use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::thread::JoinHandle;

//#[derive(Copy)]
enum Message {
    Command(String),
    Data(Box<Any + Send>),
}

type ActorRef = Arc<Mutex<Actor>>;

trait Actor {
    fn receive(&mut self, Arc<Message>);
    fn handle_message(&mut self);
    fn send_message(&self, actor_ref: ActorRef, message: Arc<Message>);
    fn broadcast(&self, message: Arc<Message>);
    // Used on dev, to be removed afterwards.
    fn send_to_first(&self, message: Arc<Message>);
}

struct Printer {
    name: String,
    // Here we use Arc, so that messages can be shared beetween actors.
    message_queue: VecDeque<Arc<Message>>,
    actor_system: Weak<ActorSystem>,
    known_actors: Vec<ActorRef>,
}

impl Printer {
    fn new(name: String, actor_system: Weak<ActorSystem>, known_actors: Vec<ActorRef>) -> Printer {
        Printer {
            name: name,
            message_queue: VecDeque::new(),
            actor_system: actor_system,
            known_actors: known_actors,
        }
    }
}

impl Actor for Printer {
    fn receive(&mut self, message: Arc<Message>) {
        self.message_queue.push_back(message);
    }

    fn handle_message(&mut self) {
        let message = self.message_queue.pop_front().unwrap().clone();

        println!("({}) treats a message", self.name);
        match *message {
            Message::Command(ref command) => {
                println!("Received Command: ({})", command)
            },
            Message::Data(ref data) => {
                match data.downcast_ref::<String>() {
                    Some(s) => println!("Received data: ({})", s),
                    None => println!("Message is dropped"),
                }
            }
        }
    }

    fn send_message(&self, actor_ref: ActorRef, message: Arc<Message>) {
        self.actor_system.upgrade().unwrap().send_to_actor(actor_ref, message);
    }

    fn send_to_first(&self, message: Arc<Message>) {
        let actor_ref = self.known_actors[0].clone();
        self.send_message(actor_ref, message);
    }

    fn broadcast(&self, message: Arc<Message>) {
        for actor_ref in &self.known_actors {
            self.send_message(actor_ref.clone(), message.clone());
        }
    }
}

struct ActorSystem {
    // TODO(gamazeps): Use an unordered container instead.
    // There is currently an issue with having an ActorRef as a Arc<Mutex<Actor + Eq + Hash>>.
    actors_table: Arc<Mutex<Vec<ActorRef>>>,
    actors_queue: Arc<Mutex<VecDeque<ActorRef>>>,
    consumer_threads: Mutex<Vec<JoinHandle<()>>>,
    myself: Mutex<Option<Weak<ActorSystem>>>,
}


impl ActorSystem {
    fn new() -> Arc<ActorSystem> {
        Arc::new(ActorSystem {
            actors_table: Arc::new(Mutex::new(Vec::new())),
            actors_queue: Arc::new(Mutex::new(VecDeque::new())),
            consumer_threads: Mutex::new(Vec::new()),
            myself: Mutex::new(None),
        })
    }

    fn init(me: Arc<ActorSystem>) {
        *me.myself.lock().unwrap() = Some(Arc::downgrade(&me));
    }


    fn myself(&self) -> Option<Weak<ActorSystem>> {
        self.myself.lock().unwrap().clone()
    }

    fn spawn_actor(&self, name: String, known_actors: Vec<ActorRef>) -> ActorRef {
        let actor_ref = Arc::new(Mutex::new(Printer::new(name, self.myself().unwrap(), known_actors)));
        {
            let mut actors_table = self.actors_table.lock().unwrap();
            actors_table.push(actor_ref.clone());
        }
        actor_ref
    }

    fn send_to_actor(&self, actor_ref: ActorRef, message: Arc<Message>) {
        {
            let mut actor = actor_ref.lock().unwrap();
            actor.receive(message);
        }
        {
            let mut actors_queue = self.actors_queue.lock().unwrap();
            actors_queue.push_back(actor_ref);
        }
    }

    fn handle_actor_message(&self) {
        let actor_ref;
        {
            actor_ref = self.actors_queue.lock().unwrap().pop_front();
        }
        if let Some(actor) = actor_ref {
            actor.lock().unwrap().handle_message();
        }
    }

    fn spawn_consumer_thread(&self) {
        let handle = thread::spawn(|| {
            println!("Spawned a thread that does nothing");
        });
        self.consumer_threads.lock().unwrap().push(handle);
    }
}

fn main() {
    let message = Arc::new(Message::Command("This is a command".to_owned()));
    let command = Arc::new(Message::Data(Box::new("This is a command".to_owned())));
    let bad_data = Arc::new(Message::Data(Box::new(1i32)));

    let actor_system = ActorSystem::new();
    ActorSystem::init(actor_system.clone());
    let actor_ref_2 = actor_system.spawn_actor("actor_2".to_owned(), Vec::new());
    let actor_ref_3 = actor_system.spawn_actor("actor_3".to_owned(), Vec::new());
    let actor_ref_1 = actor_system.spawn_actor(
        "actor_1".to_owned(), vec![actor_ref_2.clone(), actor_ref_3.clone()]);

    {
        let actor = actor_ref_1.lock().unwrap();
        actor.broadcast(message.clone());
        actor.broadcast(command.clone());
        actor.broadcast(bad_data.clone());
    }

    actor_system.spawn_consumer_thread();

    loop {
        actor_system.handle_actor_message();
    }

}
