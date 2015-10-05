use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

struct Actor {
    // VecDeque is the current recommendation for FIFOs.
    // Right now this is a simple FIFO for messages to be printed.
    messages_queue: VecDeque<String>,
}

impl Actor {
    fn new(messages: VecDeque<String>) -> Actor {
        Actor {
           messages_queue: messages,
        }
    }

    // Right now the send and treat_messages methods are not in the same structs, which is a bad
    // API, this will be changed once the main features are working.
    fn treat_messages(&mut self) {
        if let Some(s) = self.messages_queue.pop_front() {
            println!("{}", s);
        }
    }
}

// The ActorWrapper struct is a way to be able to add Actors to the FIFO while staying safe and
// clean.
struct ActorWrapper {
    // Since the FIFO will be acccessed in a concurrent manner, wrapping it around an Arc and Mutex
    // is needed.
    // This FIFO contains Actors wrapped in the same way for the same reason.
    worker_queue: Arc<Mutex<VecDeque<Arc<Mutex<Actor>>>>>,
    // Having an inner actor is needed as it is extremely messy to have the actor add itself to the
    // FIFO.
    // The Actor is put in an Arc to avoid dangling pointers, and the Mutex is there for concurrent
    // mutability.
    actor: Arc<Mutex<Actor>>,
}

impl ActorWrapper {
    fn new(
        actor: Arc<Mutex<Actor>>,
        queue: Arc<Mutex<VecDeque<Arc<Mutex<Actor>>>>>) -> ActorWrapper {
        ActorWrapper {
            worker_queue: queue,
            actor: actor,
        }
    }

    fn send(&mut self, message: String) {
        {
            let mut actor = self.actor.lock().unwrap();
            actor.messages_queue.push_back(message);
        }

        let mut queue = self.worker_queue.lock().unwrap();
        queue.push_back(self.actor.clone());
    }
}

fn main() {
    let shared_queue: Arc<Mutex<VecDeque<_>>> = Arc::new(Mutex::new(VecDeque::new()));

    let mut actor_1 = ActorWrapper::new(
        Arc::new(Mutex::new(Actor::new(VecDeque::new()))),
        shared_queue.clone());
    let mut actor_2 = ActorWrapper::new(
        Arc::new(Mutex::new(Actor::new(VecDeque::new()))),
        shared_queue.clone());

    // The messages should be printed in this order.
    actor_1.send("Hello world 1-1!".to_string());
    actor_2.send("Hello world 2!".to_string());
    actor_1.send("Hello world 1-2!".to_string());

    loop {
        let mut queue = shared_queue.lock().unwrap();
        if let Some(actor) = queue.pop_front() {
            actor.lock().unwrap().treat_messages();
        }
    }
}
