extern crate eventual;
extern crate rand;
extern crate robots;

use eventual::Async;
use rand::Rng;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Arguments, Props};

#[derive(Copy, Clone, PartialEq)]
enum Exchanges {
    Request,
    Answer(u32),
}

struct Requester;

impl Actor for Requester {
    fn receive<Args: Arguments>(&self, _message: Box<Any>, _context: ActorCell<Args, Requester>) {}
}

impl Requester {
    fn new(_dummy: ()) -> Requester {
        Requester
    }
}

struct Answerer {
    secret: u32,
}

impl Actor for Answerer {
    fn receive<Args: Arguments>(&self,
                                message: Box<Any>,
                                context: ActorCell<Args, Answerer>) {
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

    let props = Props::new(Arc::new(Requester::new), ());
    let requester = actor_system.actor_of(props, "requester".to_owned());
    let props = Props::new(Arc::new(Answerer::new), ());
    let answerer = actor_system.actor_of(props, "answerer".to_owned());

    // FIXME(gamazeps): eventual futures seem to be a bad idea, as we have to await with them.
    // And that kinda beats the whole point of having futures.
    let future = requester.ask_to(answerer, Exchanges::Request);
    let _result = future.and_then(|v| {
                            if let Exchanges::Answer(secret) = v {
                                println!("{}", secret)
                            }
                        })
                        .await();

    std::thread::sleep(Duration::from_millis(100));
    actor_system.shutdown();
}
