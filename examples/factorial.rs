/// In this example we show how to compute a factorial with actors.
/// Here the actor simply sends messages to himself.

extern crate robots;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Props};

/// Basic tail recursive factorial.
struct Factorial;

impl Actor for Factorial {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<(u32, u32)>(message) {
            let (i, j) = *message;
            if i == 0 {
                println!("factorial: {}", j);
            } else {
                context.tell(context.actor_ref(), (i - 1, j * i));
            }
        }
    }
}

impl Factorial {
    fn new(_dummy: ()) -> Factorial {
        Factorial
    }
}

fn main() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(2);

    let props_factorial = Props::new(Arc::new(Factorial::new), ());
    let factorial_actor_ref_1 = actor_system.actor_of(props_factorial.clone(), "sender".to_owned());
    let factorial_actor_ref_2 = actor_system.actor_of(props_factorial.clone(),
                                                      "receiver".to_owned());

    factorial_actor_ref_1.tell_to(factorial_actor_ref_2.clone(), (3u32, 1u32));
    factorial_actor_ref_1.tell_to(factorial_actor_ref_2.clone(), (7u32, 1u32));
    factorial_actor_ref_1.tell_to(factorial_actor_ref_2.clone(), (11u32, 1u32));

    std::thread::sleep(Duration::from_millis(100));
    actor_system.shutdown();
}
