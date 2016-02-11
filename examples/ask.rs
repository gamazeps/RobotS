/// In this example we show how to use the ask pattern.
/// The ask pattern consists of sending a message expecting an answer, the answer is then put in
/// a Future.
/// We chose to use it from the "outside" here, but it works as well inside an actor by calling
/// `context.ask(target, message)`.

extern crate env_logger;
extern crate rand;
extern crate robots;

use rand::Rng;

use std::any::Any;
use std::sync::Arc;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Props};

#[derive(Copy, Clone, PartialEq)]
enum Exchanges {
    Request,
    Answer(u32),
}

struct Answerer {
    secret: u32,
}

impl Actor for Answerer {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<Exchanges>(message) {
            if *message == Exchanges::Request {
                context.complete(context.sender(), Exchanges::Answer(self.secret));
            }
        }
    }
}

impl Answerer {
    fn new(_dummy: ()) -> Answerer {
        Answerer { secret: rand::thread_rng().gen_range(1, 101) }
    }
}

fn main() {
    env_logger::init().unwrap();
    let actor_system = ActorSystem::new("test".to_owned());
    println!("system started");
    actor_system.spawn_threads(1);

    let props = Props::new(Arc::new(Answerer::new), ());
    let answerer = actor_system.actor_of(props, "answerer".to_owned());

    let future = actor_system.ask(answerer, Exchanges::Request, "request".to_owned());
    let x: Exchanges = actor_system.extract_result(future);

    match x {
        Exchanges::Request => unreachable!(),
        Exchanges::Answer(value) => println!("The secret value is {}", value),
    }

    actor_system.shutdown();
}
