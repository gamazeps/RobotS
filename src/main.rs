use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

enum Message {
    Command(String),
    Data(Box<Any + Send>),
}

type ActorRef = Arc<Mutex<Actor>>;

trait Actor {
    fn receive(&mut self, Message);
    fn handle_message(&mut self);
}

struct Printer {
    _name: String,
    message_queue: VecDeque<Message>,
}

impl Printer {
    fn new(name: String) -> Printer {
        Printer {
            _name: name,
            message_queue: VecDeque::new(),
        }
    }
}

impl Actor for Printer {
    fn receive(&mut self, message: Message) {
        self.message_queue.push_back(message);
    }

    fn handle_message(&mut self) {
        let message = self.message_queue.pop_front().unwrap();

        match message {
            Message::Command(command) => {
                println!("Received Command: ({})", command)
            },
            Message::Data(truc) => {
                match truc.downcast_ref::<String>() {
                    Some(s) => println!("Received data: ({})", s),
                    None => println!("Message is dropped"),
                }
            }
        }
    }
}

struct ActorSystem {
    // TODO(gamazeps): Use an unordered container instead.
    // There is currently an issue with having an ActorRef as a Arc<Mutex<Actor + Eq + Hash>>.
    actors_table: Mutex<Vec<ActorRef>>,
    actors_queue: Arc<Mutex<VecDeque<ActorRef>>>,
}


impl ActorSystem {
    fn new() -> ActorSystem {
        ActorSystem {
            actors_table: Mutex::new(Vec::new()),
            actors_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn spawn_actor(&self, name: String) -> ActorRef {
        let actor_ref = Arc::new(Mutex::new(Printer::new(name)));
        {
            let mut actors_table = self.actors_table.lock().unwrap();
            actors_table.push(actor_ref.clone());
        }
        actor_ref
    }

    fn send_to_actor(&self, actor_ref: ActorRef, message: Message) {
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
}

fn main() {
    let message = "This is a message".to_string();
    let command = "This is a command".to_string();

    let actor_system = ActorSystem::new();
    let actor_ref = actor_system.spawn_actor("actor_1".to_string());

    actor_system.send_to_actor(actor_ref.clone(), Message::Command(command));
    actor_system.send_to_actor(actor_ref.clone(), Message::Data(Box::new(message)));
    actor_system.send_to_actor(actor_ref.clone(), Message::Data(Box::new(3i32)));

    actor_system.handle_actor_message();
    actor_system.handle_actor_message();
    actor_system.handle_actor_message();
}
