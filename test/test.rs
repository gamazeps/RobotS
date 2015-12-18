extern crate eventual;
extern crate robots;

use eventual::Async;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

use robots::actors::{Actor, ActorSystem, ActorCell, Arguments, ActorContext, Props};

#[derive(Debug, PartialEq)]
enum Res {
    Ok,
    Err
}

#[derive(Copy, Clone)]
enum InternalStateMessage {
    Set(u32),
    Get,
    Panic,
}

struct InternalState {
    last: Mutex<u32>,
    sender: Arc<Mutex<Sender<Res>>>,
}

impl Actor<InternalStateMessage> for InternalState {
    fn receive<Args: Arguments>(&self, message: InternalStateMessage, context: ActorCell<Args, InternalStateMessage, InternalState>) {
        match message {
            InternalStateMessage::Get => context.tell(context.sender(), *self.last.lock().unwrap()),
            InternalStateMessage::Set(message) => {
                // Here mixing the test actir for the two tests might seem a bit weird,
                // but we would get two very similar actors otherwise.
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
            InternalStateMessage::Panic => panic!(""),
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
        actor_ref_1.tell_to(actor_ref_2.clone(), InternalStateMessage::Set(i as u32));
    }

    let res = rx.recv();
    assert_eq!(Ok(Res::Ok), res);

    actor_system.shutdown();
}

#[test]
fn recover_from_panic() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);

    let (tx, _rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(InternalState::new), tx);
    let requester = actor_system.actor_of(props.clone(), "sender".to_owned());
    let answerer = actor_system.actor_of(props.clone(), "receiver".to_owned());

    requester.tell_to(answerer.clone(), InternalStateMessage::Set(10));
    let res: u32 = requester.ask_to::<InternalStateMessage, u32, ()>(answerer.clone(), InternalStateMessage::Get).and_then(|x| Ok(x)).await().unwrap();
    assert_eq!(10u32, res);

    requester.tell_to(answerer.clone(), InternalStateMessage::Panic);
    let res: u32 = requester.ask_to::<InternalStateMessage, u32, ()>(answerer.clone(), InternalStateMessage::Get).and_then(|x| Ok(x)).await().unwrap();
    assert_eq!(0u32, res);

    actor_system.shutdown();
}
