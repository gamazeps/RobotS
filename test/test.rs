extern crate robots;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

use robots::actors::{Actor, ActorSystem, ActorCell, Arguments, ActorContext, Props};

#[derive(Debug, PartialEq)]
enum Res {
    Ok,
    Err
}

struct InternalState {
    last: Mutex<u32>,
    sender: Arc<Mutex<Sender<Res>>>,
}

impl Actor<u32> for InternalState {
    fn receive<Args: Arguments>(&self, message: u32, _context: ActorCell<Args, u32, InternalState>) {
        let mut last = self.last.lock().unwrap();
        if message <= *last {
            let _ = self.sender.lock().unwrap().send(Res::Err);
        } else {
            *last = message;
        }
        if *last == 1000 {
            let _ = self.sender.lock().unwrap().send(Res::Ok);
        }
    }
}

impl InternalState {
    fn new(sender: Arc<Mutex<Sender<Res>>>) -> InternalState {
        InternalState {
            last: Mutex::new(0),
            sender: sender,
        }
    }
}

#[test]
fn read_messages_in_order() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(9);

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(InternalState::new), tx);
    let actor_ref_1 = actor_system.actor_of(props.clone(), "sender".to_owned());
    let actor_ref_2 = actor_system.actor_of(props.clone(), "receiver".to_owned());

    for i in 1..1001 {
        actor_ref_1.tell_to(actor_ref_2.clone(), i as u32);
    }

    let res = rx.recv();
    assert_eq!(Ok(Res::Ok), res);

    actor_system.shutdown();
}
