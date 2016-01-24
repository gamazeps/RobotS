extern crate robots;

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, ActorRef, Props};

#[derive(Debug, PartialEq)]
enum Res {
    Ok,
    Err,
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

impl Actor for InternalState {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<InternalStateMessage>(message) {
            match *message {
                InternalStateMessage::Get => {
                    context.tell(context.sender(), *self.last.lock().unwrap())
                }
                InternalStateMessage::Set(message) => {
                    // Here mixing the test actor for the two tests might seem a bit weird,
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
    let res = answerer.ask(InternalStateMessage::Get).await().unwrap();
    let res = Box::<Any>::downcast::<u32>(res).unwrap();
    assert_eq!(10u32, *res);

    requester.tell_to(answerer.clone(), InternalStateMessage::Panic);
    let res = answerer.ask(InternalStateMessage::Get).await().unwrap();
    let res = Box::<Any>::downcast::<u32>(res).unwrap();
    assert_eq!(0u32, *res);

    actor_system.shutdown();
}

struct Resolver;

impl Actor for Resolver {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<String>(message) {
            let res = context.identify_actor(*message)
                             .await()
                             .unwrap();
            context.tell(context.sender(), res);
        }
    }
}

impl Resolver {
    fn new(_dummy: ()) -> Resolver {
        Resolver
    }
}

#[test]
fn resolve_name_real_path() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(2);

    let props = Props::new(Arc::new(Resolver::new), ());
    let answerer = actor_system.actor_of(props.clone(), "answerer".to_owned());
    let requester = actor_system.actor_of(props.clone(), "sender".to_owned());

    // We wait to be sure that the actors will be registered to the name resolver.
    std::thread::sleep(Duration::from_millis(100));

    let res = answerer.ask("/user/sender".to_owned()).await().unwrap();
    let res = Box::<Any>::downcast::<Option<ActorRef>>(res).unwrap();
    assert_eq!(requester.path(), (*res).unwrap().path());

    actor_system.shutdown();
}

#[test]
fn resolve_name_fake_path() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(2);

    let props = Props::new(Arc::new(Resolver::new), ());
    let answerer = actor_system.actor_of(props.clone(), "answerer".to_owned());

    // We wait to be sure that the actors will be registered to the name resolver.
    std::thread::sleep(Duration::from_millis(100));

    let res = answerer.ask("/foo/bar".to_owned()).await().unwrap();
    let res = *Box::<Any>::downcast::<Option<ActorRef>>(res).unwrap();

    match res {
        None => {}
        Some(_) => panic!("The name resolver gave an ActorRef when he should not."),
    };

    actor_system.shutdown();
}

// This actor simply answers twice with () when send a message.
// He also sends () through a channel when restarted.
struct DoubleAnswer {
    sender: Arc<Mutex<Sender<()>>>,
}

impl Actor for DoubleAnswer {
    fn post_restart(&self, _context: ActorCell) {
        let sender = self.sender.lock().unwrap();
        let _res = sender.send(());
    }

    fn receive(&self, _message: Box<Any>, context: ActorCell) {
        context.tell(context.sender(), ());
        context.tell(context.sender(), ());
    }
}

impl DoubleAnswer {
    fn new(sender: Arc<Mutex<Sender<()>>>) -> DoubleAnswer {
        DoubleAnswer {
            sender: sender
        }
    }
}


#[test]
fn ask_answer_twice() {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(DoubleAnswer::new), tx);
    let answerer = actor_system.actor_of(props.clone(), "answerer".to_owned());

    let res = answerer.ask(()).await().unwrap();
    let res = *Box::<Any>::downcast::<()>(res).unwrap();
    // We chack that the value contained in the future is the correct one.
    assert_eq!(res, ());

    // We check that the actor has been restarted because of a panic (reason is not caught yet).
    let res = rx.recv();
    assert_eq!(res, Ok(()));

    std::thread::sleep(Duration::from_millis(10));

    actor_system.shutdown();
}

// This actor simply answers twice with () when send a message.
// He also sends () through a channel when restarted.
struct DoubleAnswer {
    sender: Arc<Mutex<Sender<()>>>,
}

impl Actor for DoubleAnswer {
    fn post_restart(&self, _context: ActorCell) {
        let mut sender = self.sender.lock().unwrap();
        sender.send(());
    }

    fn receive(&self, _message: Box<Any>, context: ActorCell) {
        context.tell(context.sender(), ());
        context.tell(context.sender(), ());
    }
}

impl DoubleAnswer {
    fn new(sender: Arc<Mutex<Sender<()>>>) -> DoubleAnswer {
        DoubleAnswer {
            sender: sender
        }
    }
}


#[test]
fn ask_answer_twice() {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(DoubleAnswer::new), tx);
    let answerer = actor_system.actor_of(props.clone(), "answerer".to_owned());

    let res = answerer.ask(()).await().unwrap();
    let res = *Box::<Any>::downcast::<()>(res).unwrap();
    // We chack that the value contained in the future is the correct one.
    assert_eq!(res, ());

    // We check that the actor has been restarted because of a panic (reason is not caught yet).
    let res = rx.recv();
    assert_eq!(res, Ok(()));

    std::thread::sleep(Duration::from_millis(10));

    actor_system.shutdown();
}
