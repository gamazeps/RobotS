use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};

enum Message<T> {
    Command(String),
    Data(T),
}

struct ActorRef<T: Send + Sized> {
    name: Arc<String>,
    message_queue: Arc<Mutex<VecDeque<Message<T>>>>,
}

impl<T: Send + Sized> ActorRef<T> {
    fn new(name: Arc<String>) -> ActorRef<T> {
        ActorRef {
            name: name,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn receive(&self, message: Message<T>) {
        self.message_queue.lock().unwrap().push_back(message);
    }

    fn handle(&self) {
        let message;
        {
            message = self.message_queue.lock().unwrap().pop_front();
        }
        let message = message.unwrap();
        match message {
            Message::Command(_) => println!("aya captain"),
            Message::Data(_) => println!("interesting data that you have"),
        }
    }

}

struct ActorSystem<T: Send + Sized> {
    actors_table: Arc<Mutex<HashMap<Arc<String>, ActorRef<T>>>>,
}

impl<T: Send + Sized> ActorSystem<T> {
    fn new() -> ActorSystem<T> {
        ActorSystem {
            actors_table: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_actor_ref(&self, handle: String) {
        let handle = Arc::new(handle);
        let actor_ref: ActorRef<T> = ActorRef::new(handle.clone());
        {
            let mut actors_table = self.actors_table.lock().unwrap();
            actors_table.insert(handle.clone(), actor_ref);
        }
    }

    fn send_message_to_actor(&self, dont_judge_me: Arc<String>, message: Message<T>) {
        let actors_table = self.actors_table.lock().unwrap();
        // Here this is very bad as get gives a ref, and thus we need to keep the lock for a long
        // time (and we also need the actor's message queue lock).
        let actor_ref = actors_table.get(&dont_judge_me.clone());
        match actor_ref {
            Some(actor_ref) => {
                println!("yay");
                actor_ref.receive(message);
                actor_ref.handle();
            }
            None => println!("oh noes"),
        }
    }
}

fn main() {
    let actor_system: ActorSystem<i32> = ActorSystem::new();
    actor_system.create_actor_ref("actor1".to_string());
    actor_system.create_actor_ref("actor2".to_string());
    actor_system.send_message_to_actor(Arc::new("actor1".to_string()), Message::Data(1i32));
    actor_system.send_message_to_actor(Arc::new("actor2".to_string()), Message::Data("ay ay"));

    println!("Hello world");
}
