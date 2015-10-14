extern crate core;

use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};

enum Message<T> {
    Command(String),
    Data(T),
}

struct ActorRef<T: Send + Sized> {
    name: String,
    message_queue: Arc<Mutex<VecDeque<Message<T>>>>,
}

trait Actor {}

impl<T: Send + Sized> ActorRef<T> {
    fn new(name: String) -> ActorRef<T> {
        ActorRef {
            name: name,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

struct ActorSystem {
    actors_table: Arc<Mutex<HashMap<String, ActorRef<Send + core::marker::Sized>>>>,
}

impl ActorSystem {
    fn new() -> ActorSystem {
        ActorSystem {
            actors_table: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_actor_ref(&self, handle: String) {
        let actor_ref: ActorRef<i32> = ActorRef::new(handle);
        {
            let actors_table = self.actors_table.lock().unwrap();
            actors_table.insert(handle, actor_ref);
        }
    }

}

fn main() {
    let _actor: ActorRef<String> = ActorRef::new("actor_1".to_string());
    println!("Hello world");
}
