#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate robots;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Props, Message};

/// Basic factorial.
struct Factorial;

impl Actor<(u32, u32)> for Factorial {
    fn receive<Args: Message>(&self, message: (u32, u32), context: ActorCell<Args, (u32, u32), Factorial>) {
        let (i, j) = message;
        if i == 0 {
            println!("factorial: {}", j);
        } else {
            context.tell(context.actor_ref(), (i - 1, j * i));
        }
    }
}

impl Factorial {
    fn new(_dummy: ()) -> Factorial {
        Factorial
    }
}

#[derive(Copy, Clone)]
enum InternalStateMessage {
    Set(u32),
    Get,
    Panic,
}

/// Basic factorial.
struct InternalState {
    counter: Mutex<u32>
}

impl Actor<InternalStateMessage> for InternalState {
    fn receive<Args: Message>(&self, message: InternalStateMessage, _context: ActorCell<Args, InternalStateMessage, InternalState>) {
        match message {
            InternalStateMessage::Get => println!("internal state: {}", *self.counter.lock().unwrap()),
            InternalStateMessage::Set(num) => *self.counter.lock().unwrap() = num,
            InternalStateMessage::Panic => panic!("actor panicked"),
        }
    }
}

impl InternalState {
    fn new(count: u32) -> InternalState {
        InternalState { counter: Mutex::new(count) }
    }
}

fn main() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(2);

    let props_factorial = Props::new(Arc::new(Factorial::new), ());
    let factorial_actor_ref = actor_system.actor_of(props_factorial.clone());

    let restarted_props = Props::new(Arc::new(InternalState::new), 3);
    let restarted_actor_ref = actor_system.actor_of(restarted_props.clone());

    restarted_actor_ref.tell_to(factorial_actor_ref.clone(), (3u32, 1u32));
    restarted_actor_ref.tell_to(factorial_actor_ref.clone(), (7u32, 1u32));
    restarted_actor_ref.tell_to(factorial_actor_ref.clone(), (11u32, 1u32));

    factorial_actor_ref.tell_to(restarted_actor_ref.clone(), InternalStateMessage::Get);
    factorial_actor_ref.tell_to(restarted_actor_ref.clone(), InternalStateMessage::Set(7));
    factorial_actor_ref.tell_to(restarted_actor_ref.clone(), InternalStateMessage::Get);

    std::thread::sleep(Duration::from_millis(1));
    factorial_actor_ref.tell_to(restarted_actor_ref.clone(), InternalStateMessage::Panic);
    std::thread::sleep(Duration::from_millis(1));
    factorial_actor_ref.tell_to(restarted_actor_ref.clone(), InternalStateMessage::Get);

    std::thread::sleep(Duration::from_millis(3));
    actor_system.terminate_threads(2);
    std::thread::sleep(Duration::from_millis(3));
    println!("Hello world!");
}
