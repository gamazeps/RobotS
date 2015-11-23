#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate robots;

use std::sync::Arc;

use robots::{Actor, ActorSystem, ActorCell, Message, Props};

struct MyActor;

impl Actor for MyActor {
    fn receive<Args: Copy + Sync + Send + 'static>(&self, message: Message, context: ActorCell<Args, MyActor>) {
        match message {
            Message::Text(s) => println!("I received a text message: ({}) !", s),
            Message::Dummy => println!("I received a dummy message !"),
        }
    }
}

impl MyActor {
    fn new(_dummy: ()) -> MyActor {
        MyActor
    }
}

fn main() {

    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);

    let props_1 = Props::new(Arc::new(MyActor::new), ());
    let actor_ref_1 = actor_system.actor_of(props_1);

    let props_2 = Props::new(Arc::new(MyActor::new), ());
    let actor_ref_2 = actor_system.actor_of(props_2);

    actor_ref_1.tell_to(actor_ref_2.clone(), Message::Dummy);
    actor_ref_1.tell_to(actor_ref_2.clone(), Message::Text("Hello there".to_owned()));

    std::thread::sleep_ms(700);
    actor_system.terminate_threads(1);
    std::thread::sleep_ms(700);

    println!("Hello world!");
}
