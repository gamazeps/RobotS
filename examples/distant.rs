extern crate robots;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, ActorPath, ActorRef, Props};

struct Dummy;

impl Actor for Dummy {
    fn pre_start(&self, context: ActorCell) {
        let distant_path = ActorPath::new_distant("/user/distant".to_owned(), "127.0.0.1:12345".to_owned());
        let distant_actor = ActorRef::new_distant(distant_path);

        context.tell(distant_actor.clone(), "(i am a dummy message)".to_owned());
        context.tell(distant_actor, 18);
    }
    fn receive(&self, _message: Box<Any>, _context: ActorCell) {}
}

impl Dummy {
    fn new(_dummy: ()) -> Dummy {
        Dummy
    }
}

fn main() {
    let actor_system = ActorSystem::new("test".to_owned());

    let props = Props::new(Arc::new(Dummy::new),());
    let _local_actor = actor_system.actor_of(props.clone(), "dummy".to_owned());

    std::thread::sleep(Duration::from_millis(10));
    actor_system.shutdown();
}

