extern crate robots;

use std::sync::Arc;
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Arguments, Props};

/// Basic factorial.
struct Factorial;

impl Actor<(u32, u32)> for Factorial {
    fn receive<Args: Arguments>(&self, message: (u32, u32), context: ActorCell<Args, (u32, u32), Factorial>) {
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

fn main() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(2);

    let props_factorial = Props::new(Arc::new(Factorial::new), ());
    let factorial_actor_ref_1 = actor_system.actor_of(props_factorial.clone(), "sender".to_owned());
    let factorial_actor_ref_2 = actor_system.actor_of(props_factorial.clone(), "receiver".to_owned());

    factorial_actor_ref_1.tell_to(factorial_actor_ref_2.clone(), (3u32, 1u32));
    factorial_actor_ref_1.tell_to(factorial_actor_ref_2.clone(), (7u32, 1u32));
    factorial_actor_ref_1.tell_to(factorial_actor_ref_2.clone(), (11u32, 1u32));

    std::thread::sleep(Duration::from_millis(10));
    actor_system.shutdown();
}
