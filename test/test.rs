extern crate env_logger;
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

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

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
                    context.complete(context.sender(), *self.last.lock().unwrap())
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
                InternalStateMessage::Panic => {
                    panic!("The actor panicked as it was asked to.")
                },
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

    let (tx, _rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(InternalState::new), tx);
    let requester = actor_system.actor_of(props.clone(), "sender".to_owned());
    let answerer = actor_system.actor_of(props.clone(), "receiver".to_owned());

    requester.tell_to(answerer.clone(), InternalStateMessage::Set(10));
    let res = actor_system.ask(answerer.clone(), InternalStateMessage::Get, "future_1".to_owned());
    let res: u32 = actor_system.extract_result(res);
    assert_eq!(10u32, res);

    requester.tell_to(answerer.clone(), InternalStateMessage::Panic);
    let res = actor_system.ask(answerer, InternalStateMessage::Get, "future_2".to_owned());
    let res: u32 = actor_system.extract_result(res);
    assert_eq!(0u32, res);

    actor_system.shutdown();
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

struct Resolver;

impl Actor for Resolver {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<String>(message) {
            let future = context.identify_actor(*message, "resolver_request".to_owned());
            context.forward_result_to_future::<Option<ActorRef>>(future, context.sender());
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

    let props = Props::new(Arc::new(Resolver::new), ());
    let answerer = actor_system.actor_of(props.clone(), "answerer".to_owned());
    let requester = actor_system.actor_of(props.clone(), "sender".to_owned());

    // We wait to be sure that the actors will be registered to the name resolver.
    std::thread::sleep(Duration::from_millis(100));

    let res = actor_system.ask(answerer, "/user/sender".to_owned(), "future".to_owned());
    println!("lol");
    let res: Option<ActorRef> = actor_system.extract_result(res);
    assert_eq!(requester.path(), res.unwrap().path());

    actor_system.shutdown();
}

#[test]
fn resolve_name_fake_path() {
    let actor_system = ActorSystem::new("test".to_owned());

    let props = Props::new(Arc::new(Resolver::new), ());
    let answerer = actor_system.actor_of(props.clone(), "answerer".to_owned());

    // We wait to be sure that the actors will be registered to the name resolver.
    std::thread::sleep(Duration::from_millis(100));

    let res = actor_system.ask(answerer, "/foo/bar".to_owned(), "future".to_owned());
    std::thread::sleep(Duration::from_millis(100));
    let res: Option<ActorRef> = actor_system.extract_result(res);

    match res {
        None => {}
        Some(_) => panic!("The name resolver gave an ActorRef when he should not."),
    };

    actor_system.shutdown();
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

struct SimpleActor1 {
    sender: Arc<Mutex<Sender<Res>>>,
}

impl Actor for SimpleActor1 {
    fn receive(&self, _message: Box<Any>, _context: ActorCell) {
        let _ = self.sender.lock().unwrap().send(Res::Ok);
    }
}

impl SimpleActor1 {
    fn new(sender: Arc<Mutex<Sender<Res>>>) -> SimpleActor1 {
        SimpleActor1 {
            sender: sender,
        }
    }
}

#[test]
fn receive_message () {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props = Props::new(Arc::new(SimpleActor1::new), tx);
    let actor_ref = actor_system.actor_of(props.clone(), "actor".to_owned());

    actor_system.tell(actor_ref, ());

    let res = rx.recv();
    assert_eq!(Ok(Res::Ok), res);

    actor_system.shutdown();
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

struct SimpleActor2 {
    sender: Arc<Mutex<Sender<Res>>>,
    state: i32,
}

impl Actor for SimpleActor2 {
    fn receive(&self, message: Box<Any>, _context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<i32>(message) {
            if *message == self.state {
                let _ = self.sender.lock().unwrap().send(Res::Ok);
            } else {
                let _ = self.sender.lock().unwrap().send(Res::Err);
            }
        }
    }
}

impl SimpleActor2 {
    fn new(args: (Arc<Mutex<Sender<Res>>>, i32)) -> SimpleActor2 {
        SimpleActor2 {
            sender: args.0,
            state: args.1,
        }
    }
}

#[test]
fn receive_correct_message () {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));
    let value = 42;

    let props = Props::new(Arc::new(SimpleActor2::new), (tx, value));
    let actor_ref = actor_system.actor_of(props.clone(), "actor".to_owned());

    actor_system.tell(actor_ref, value);

    let res = rx.recv();
    assert_eq!(Ok(Res::Ok), res);

    actor_system.shutdown();
}
