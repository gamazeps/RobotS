#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate robots;

use std::sync::{Arc};

use robots::{Actor, ActorSystem, ActorCell, ActorContext, Message, Props};

struct MyActor;

impl Actor for MyActor {
    fn receive<Args: Copy + Sync + Send + 'static>(&self, message: Message, context: ActorCell<Args, MyActor>) {
        match message {
            Message::Text(s) => println!("I received a text message: ({}) !", s),
            Message::Dummy => println!("I received a dummy message !"),
            _ => println!("I don't know what i received"),
        }
    }
}

impl MyActor {
    fn new(_dummy: ()) -> MyActor {
        MyActor
    }
}

/// Basic factorial.
struct Factorial;

impl Actor for Factorial {
    fn receive<Args: Copy + Sync + Send + 'static>(&self, message: Message, context: ActorCell<Args, Factorial>) {
        match message {
            Message::Numbers(i, j) => {
                    if i == 0 {
                        println!("factorial: {}", j);
                    }
                    else {
                        context.tell(context.actor_ref(), Message::Numbers(i - 1, j * i));
                    }
            }
            _ => println!("Send me NUMBERSS !!"),
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
    actor_system.spawn_threads(5);

    let props_1 = Props::new(Arc::new(Factorial::new), ());
    let actor_ref_1 = actor_system.actor_of(props_1);

    let props_2 = Props::new(Arc::new(MyActor::new), ());
    let actor_ref_2 = actor_system.actor_of(props_2);

    actor_ref_1.tell_to(actor_ref_2.clone(), Message::Dummy);
    actor_ref_1.tell_to(actor_ref_2.clone(), Message::Text("Hello there".to_owned()));

    actor_ref_2.tell_to(actor_ref_1.clone(), Message::Numbers(3, 1));
    actor_ref_2.tell_to(actor_ref_1.clone(), Message::Numbers(7, 1));
    actor_ref_2.tell_to(actor_ref_1.clone(), Message::Numbers(11, 1));

    std::thread::sleep_ms(100);
    actor_system.terminate_threads(5);
    std::thread::sleep_ms(100);

    println!("Hello world!");
}
