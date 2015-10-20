use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};

// #[derive(Clone)]
enum Message {
    Command(String),
    Data(Box<Any + Send>),
}

type ActorRef = Arc<Mutex<Actor>>;

trait Actor {
    fn receive(&mut self, Message);
    fn handle_message(&mut self);
    fn send_message(&self, actor_ref: ActorRef, message: Message);
    fn broadcast(&self, message: Message);
    fn send_to_first(&self, message: Message);
}

struct Printer {
    _name: String,
    message_queue: VecDeque<Message>,
    actor_system: Weak<ActorSystem>,
    known_actors: Vec<ActorRef>,
}

impl Printer {
    fn new(name: String, actor_system: Weak<ActorSystem>, known_actors: Vec<ActorRef>) -> Printer {
        Printer {
            _name: name,
            message_queue: VecDeque::new(),
            actor_system: actor_system,
            known_actors: known_actors,
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

    fn send_message(&self, actor_ref: ActorRef, message: Message) {
        self.actor_system.upgrade().unwrap().send_to_actor(actor_ref, message);
    }

    fn send_to_first(&self, message: Message) {
        let actor_ref = self.known_actors[0].clone();
        self.send_message(actor_ref, message);
    }

    fn broadcast(&self, message: Message) {
        // TODO(gamazeps): This should use Arc<Message> instead of regular Message, it may imply to
        // change this all over the code.
        //for actor_ref in self.known_actors.iter() {
        //    self.send_message(actor_ref.clone(), message.clone());
        //}
    }
}

struct ActorSystem {
    // TODO(gamazeps): Use an unordered container instead.
    // There is currently an issue with having an ActorRef as a Arc<Mutex<Actor + Eq + Hash>>.
    actors_table: Mutex<Vec<ActorRef>>,
    actors_queue: Arc<Mutex<VecDeque<ActorRef>>>,
    myself: Mutex<Option<Weak<ActorSystem>>>,
}


impl ActorSystem {
    fn new() -> Arc<ActorSystem> {
        Arc::new(ActorSystem {
            actors_table: Mutex::new(Vec::new()),
            actors_queue: Arc::new(Mutex::new(VecDeque::new())),
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
    ActorSystem::init(actor_system.clone());
    let actor_ref_2 = actor_system.spawn_actor("actor_2".to_string(), Vec::new());
    let actor_ref_1 = actor_system.spawn_actor("actor_1".to_string(), vec![actor_ref_2.clone()]);

    {
        let actor = actor_ref_1.lock().unwrap();
        actor.send_to_first(Message::Command(command));
        actor.send_to_first(Message::Data(Box::new(message)));
        actor.send_to_first(Message::Data(Box::new(3i32)));
    }

    actor_system.handle_actor_message();
    actor_system.handle_actor_message();
    actor_system.handle_actor_message();
}
