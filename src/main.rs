#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate robots;

use std::sync::{Arc};
use std::time::Duration;

use robots::{Actor, ActorSystem, ActorCell, ActorContext, Props};

/// Basic factorial.
struct Factorial;

impl Actor<(u32, u32)> for Factorial {
    fn receive<Args: Copy + Sync + Send + 'static>(&self, message: (u32, u32), context: ActorCell<Args, (u32, u32), Factorial>) {
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
    actor_system.spawn_threads(1);

    let props = Props::new(Arc::new(Factorial::new), ());
    let actor_ref_1 = actor_system.actor_of(props.clone());

    let actor_ref_2 = actor_system.actor_of(props.clone());

    actor_ref_2.tell_to(actor_ref_1.clone(), (3u32, 1u32));
    actor_ref_2.tell_to(actor_ref_1.clone(), (7u32, 1u32));
    actor_ref_2.tell_to(actor_ref_1.clone(), (11u32, 1u32));

    std::thread::sleep(Duration::from_millis(1));
    actor_system.terminate_threads(1);
    std::thread::sleep(Duration::from_millis(1));

    println!("Hello world!");
}
