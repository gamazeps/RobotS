extern crate robots;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Arguments, Props};

#[derive(Copy, Clone, PartialEq)]
enum Greetings {
    Greet,
    Done,
}

struct HelloWorld;

impl Actor for HelloWorld {
    fn pre_start<Args: Arguments>(&self, context: ActorCell<Args, HelloWorld>) {
        let props = Props::new(Arc::new(Greeter::new), ());
        let greeter = context.actor_of(props, "greeter".to_owned());
        context.tell(greeter, Greetings::Greet);
    }

    fn receive<Args: Arguments>(&self,
                                message: Box<Any>,
                                context: ActorCell<Args, HelloWorld>) {
        if let Ok(message) = Box::<Any>::downcast::<Greetings>(message) {
            if *message == Greetings::Done {
                context.stop(context.sender());
            }
        }
    }
}

impl HelloWorld {
    fn new(_dummy: ()) -> HelloWorld {
        HelloWorld
    }
}

struct Greeter;

impl Actor for Greeter {
    fn receive<Args: Arguments>(&self,
                                message: Box<Any>,
                                context: ActorCell<Args, Greeter>) {
        if let Ok(message) = Box::<Any>::downcast::<Greetings>(message) {
            if *message == Greetings::Greet {
                println!("Hello World");
                context.tell(context.sender(), Greetings::Done);
            }
        }
    }
}

impl Greeter {
    fn new(_dummy: ()) -> Greeter {
        Greeter
    }
}

fn main() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);

    let props = Props::new(Arc::new(HelloWorld::new), ());
    let _actor = actor_system.actor_of(props, "hello_world".to_owned());

    std::thread::sleep(Duration::from_millis(10));
    actor_system.shutdown();
}
