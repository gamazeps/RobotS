#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate robots;

use robots::{Actor, ActorRef, ActorSystem, Message, Props};

struct MyActor {
    x: i32
}

impl Actor for MyActor {
    fn receive(&self, message: Message) {}
}

impl MyActor {
    fn new(x: i32) -> MyActor {
        MyActor {x: x}
    }
}

fn main() {

    let actor_system = ActorSystem::new("test".to_owned());

    let props = Props::new(Box::new(MyActor::new), 2);
    let actor_ref = actor_system.actor_of(props);

    println!("Hello world!");
}
