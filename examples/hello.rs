/// This example is a very simple hello world (the same you can see in
/// [akka](https://github.com/akka/akka/blob/v2.0/akka-samples/akka-sample-hello/src/main/scala/sample/hello/Main.scala)
/// Here we create a HelloWorld actor that will create a Greeter actor, send him a greeting
/// request, handle the greeting and then terminate the Greeter.
///
/// This shows how to stop actors, use the pre_start method and basic message sending.

extern crate env_logger;
extern crate robots;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Props};

#[derive(Copy, Clone, PartialEq)]
enum Greetings {
    Greet,
    Done,
}

struct HelloWorld;

impl Actor for HelloWorld {
    fn pre_start(&self, context: ActorCell) {
        let props = Props::new(Arc::new(Greeter::new), ());
        let greeter = context.actor_of(props, "greeter".to_owned()).unwrap();
        context.tell(greeter, Greetings::Greet);
    }

    fn receive(&self, message: Box<Any>, context: ActorCell) {
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
    fn receive(&self, message: Box<Any>, context: ActorCell) {
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
    env_logger::init().unwrap();
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);

    let props = Props::new(Arc::new(HelloWorld::new), ());
    let _actor = actor_system.actor_of(props, "hello_world".to_owned());

    std::thread::sleep(Duration::from_millis(100));
    actor_system.shutdown();
}
