extern crate robots;

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, Props};

struct Counter {
    counter: Mutex<u32>,
}

impl Actor for Counter {
    fn receive(&self, _message: Box<Any>, _context: ActorCell) {
        let mut count = self.counter.lock().unwrap();
        *count += 1;
        println!("count: {}", *count);
    }
}

impl Counter {
    fn new(_dummy: ()) -> Counter {
        Counter {
            counter: Mutex::new(0)
        }
    }
}

fn main() {
    let actor_system = ActorSystem::new("counter".to_owned());

    let props = Props::new(Arc::new(Counter::new),());
    let actor_ref_1 = actor_system.actor_of(props.clone(), "counter".to_owned());
    let actor_ref_2 = actor_system.actor_of(props.clone(), "sender".to_owned());

    actor_ref_1.tell_to(actor_ref_2.clone(), ());
    actor_ref_1.tell_to(actor_ref_2.clone(), ());
    actor_ref_1.tell_to(actor_ref_2.clone(), ());

    std::thread::sleep(Duration::from_millis(100));
    actor_system.shutdown();
}
