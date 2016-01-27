/// In this example we show how to use the ask pattern.
/// The ask pattern consists of sending a message expecting an answer, the answer is then put in
/// a Future (we are using carllerche's eventual for now).
/// We chose to use it from the "outside" here, but it works as well inside an actor by calling
/// `context.ask(target, message)`.

extern crate rand;
extern crate robots;

use rand::Rng;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

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
                context.tell(context.sender(), Exchanges::Answer(self.secret));
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
    let actor_system = ActorSystem::new("test".to_owned());
    println!("system started");
    actor_system.spawn_threads(1);

    let props = Props::new(Arc::new(Answerer::new), ());
    let answerer = actor_system.actor_of(props, "answerer".to_owned());

    // FIXME(gamazeps): eventual futures seem to be a bad idea, as we have to await with them.
    // And that kinda beats the whole point of having futures.
    let future = actor_system.ask(answerer, Exchanges::Request, "request".to_owned());
    let result = future.await().unwrap();
    if let Ok(result) = Box::<Any>::downcast::<Exchanges>(result) {
        if let Exchanges::Answer(secret) = *result {
            println!("{}", secret)
        }
    }

    std::thread::sleep(Duration::from_millis(100));
    actor_system.shutdown();
}
