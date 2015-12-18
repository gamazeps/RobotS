#![feature(test)]

extern crate robots;
extern crate test;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Arguments, Props};

use test::Bencher;

struct InternalState {
    counter: Mutex<u32>,
    sender: Arc<Mutex<Sender<()>>>,
}

impl Actor<()> for InternalState {
    fn receive<Args: Arguments>(&self, _message: (), _context: ActorCell<Args, (), InternalState>) {
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        if *counter >= 1_000_000 {
            let _ = self.sender.lock().unwrap().send(());
        }
    }
}

impl InternalState {
    fn new(sender: Arc<Mutex<Sender<()>>>) -> InternalState {
        InternalState {
            counter: Mutex::new(0),
            sender: sender,
        }
    }
}


#[bench]
fn send_1000_messages(b: &mut Bencher) {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(InternalState::new), tx);
    let actor_ref_1 = actor_system.actor_of(props.clone(), "sender".to_owned());
    let actor_ref_2 = actor_system.actor_of(props.clone(), "receiver".to_owned());

    b.iter(|| {
        for _ in 0..1_000_001 {
            actor_ref_1.tell_to(actor_ref_2.clone(), ());
        }
        let _ = rx.recv();
    });

    actor_system.shutdown();
}
