#![feature(test)]

extern crate robots;
extern crate test;

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Props};

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
    fn receive(&self, message: Box<Any>, _context: ActorCell) {
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
/// This bench sends a thousand messages to an actor then waits for an answer on a channel.
/// When the thousandth is handled the actor sends a message on the above channel.
fn send_1000_messages(b: &mut Bencher) {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(InternalState::new), tx);
    let actor_ref_1 = actor_system.actor_of(props.clone(), "sender".to_owned());
    let actor_ref_2 = actor_system.actor_of(props.clone(), "receiver".to_owned());

    b.iter(|| {
        for _ in 0..999 {
            actor_ref_1.tell_to(actor_ref_2.clone(), BenchMessage::Nothing);
        }
        actor_ref_1.tell_to(actor_ref_2.clone(), BenchMessage::Over);
        let _ = rx.recv();
    });

    actor_system.shutdown();
}

struct Dummy;

impl Actor for Dummy {
    fn receive(&self, _message: Box<Any>, _context: ActorCell) {}
}

impl Dummy {
    fn new(_: ()) -> Dummy {
        Dummy
    }
}

#[bench]
/// This bench creates a thousand empty actors.
/// Since actor creation is synchronous this is ok to just call the function mutiple times.
/// The created actor is empty in order to just bench the overhead of creation.
fn create_1000_actors(b: &mut Bencher) {
    let actor_system = ActorSystem::new("test".to_owned());

    let props = Props::new(Arc::new(Dummy::new), ());

    b.iter(|| {
        for i in 0..1_000 {
            actor_system.actor_of(props.clone(), format!("{}", i));
        }
    });

    actor_system.shutdown();
}
