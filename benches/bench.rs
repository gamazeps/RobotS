#![feature(test)]

extern crate robots;
extern crate test;

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Arguments, Props};

use test::Bencher;

#[derive(Copy, Clone, PartialEq)]
enum BenchMessage {
    Nothing,
    Over,
}

struct InternalState {
    sender: Arc<Mutex<Sender<()>>>,
}

impl Actor for InternalState {
    fn receive<Args: Arguments>(&self,
                                message: Box<Any>,
                                _context: ActorCell<Args, InternalState>) {
        if let Ok(message) = Box::<Any>::downcast::<BenchMessage>(message) {
            if *message == BenchMessage::Over {
                let _ = self.sender.lock().unwrap().send(());
            }
        }
    }
}

impl InternalState {
    fn new(sender: Arc<Mutex<Sender<()>>>) -> InternalState {
        InternalState { sender: sender }
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
        for _ in 0..1_000 {
            actor_ref_1.tell_to(actor_ref_2.clone(), BenchMessage::Nothing);
        }
        actor_ref_1.tell_to(actor_ref_2.clone(), BenchMessage::Over);
        let _ = rx.recv();
    });

    actor_system.shutdown();
}

struct Dummy;

impl Actor for Dummy {
    fn receive<Args: Arguments>(&self, _message: Box<Any>, _context: ActorCell<Args, Dummy>) {}
}

impl Dummy {
    fn new(_: ()) -> Dummy {
        Dummy
    }
}

#[bench]
fn create_1000_actors(b: &mut Bencher) {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);

    let props = Props::new(Arc::new(Dummy::new), ());

    b.iter(|| {
        for i in 0..1_000 {
            actor_system.actor_of(props.clone(), format!("{}", i));
        }
    });

    actor_system.shutdown();
}
