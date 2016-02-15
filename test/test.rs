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
    env_logger::init().unwrap();
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

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

struct SimpleActor3;

impl Actor for SimpleActor3 {
    fn receive(&self, _message: Box<Any>, _context: ActorCell) {
        panic!("Panic as planned, should start an failure handler.");
    }
}

impl SimpleActor3 {
    fn new(_args: ()) -> SimpleActor3 {
        SimpleActor3
    }
}

#[derive(Clone)]
enum SimpleActor4Messages {
    RegisterMe,
    ActorFailure,
}

struct SimpleActor4 {
    sender: Arc<Mutex<Sender<Res>>>,
}

impl Actor for SimpleActor4 {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<SimpleActor4Messages>(message) {
            match *message {
                SimpleActor4Messages::RegisterMe => {
                    context.monitor(context.sender(), Arc::new(|_failure, context| {
                        // When the monitored actor failed, the monitorer pipes a message to itself.
                        context.tell(context.actor_ref(), SimpleActor4Messages::ActorFailure);
                    }));
                    context.tell(context.sender(), ());
                },
                SimpleActor4Messages::ActorFailure => {
                    let _ = self.sender.lock().unwrap().send(Res::Ok);
                }
            }
        }
    }
}

impl SimpleActor4 {
    fn new(args: Arc<Mutex<Sender<Res>>>) -> SimpleActor4 {
        SimpleActor4 {
            sender: args
        }
    }
}

#[test]
fn receive_failure_notifications () {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props_panicker = Props::new(Arc::new(SimpleActor3::new), ());
    let props_handler = Props::new(Arc::new(SimpleActor4::new), tx);
    let panicker = actor_system.actor_of(props_panicker.clone(), "panicker".to_owned());
    let handler = actor_system.actor_of(props_handler.clone(), "handler".to_owned());

    panicker.tell_to(handler.clone(), SimpleActor4Messages::RegisterMe);

    let res = rx.recv();
    assert_eq!(Ok(Res::Ok), res);

    actor_system.shutdown();
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
struct SimpleActor5;

impl Actor for SimpleActor5 {
    fn receive(&self, _message: Box<Any>, context: ActorCell) {
        context.fail("failure");
    }
}

impl SimpleActor5 {
    fn new(_args: ()) -> SimpleActor5 {
        SimpleActor5
    }
}

#[derive(Clone)]
enum SimpleActor6Messages {
    RegisterMe,
    ActorFailureOk,
    ActorFailureErr,
}

struct SimpleActor6 {
    sender: Arc<Mutex<Sender<Res>>>,
}

impl Actor for SimpleActor6 {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<SimpleActor6Messages>(message) {
            match *message {
                SimpleActor6Messages::RegisterMe => {
                    context.monitor(context.sender(), Arc::new(|failure, context| {
                        // When the monitored actor failed, the monitorer pipes a message to itself.
                        match failure.reason() {
                            "failure" => context.tell(context.actor_ref(), SimpleActor6Messages::ActorFailureOk),
                            _ => context.tell(context.actor_ref(), SimpleActor6Messages::ActorFailureErr),
                        };
                    }));
                    context.tell(context.sender(), ());
                },
                SimpleActor6Messages::ActorFailureOk => {
                    let _ = self.sender.lock().unwrap().send(Res::Ok);
                },
                SimpleActor6Messages::ActorFailureErr => {
                    let _ = self.sender.lock().unwrap().send(Res::Err);
                },
            }
        }
    }
}

impl SimpleActor6 {
    fn new(args: Arc<Mutex<Sender<Res>>>) -> SimpleActor6 {
        SimpleActor6 {
            sender: args
        }
    }
}

#[test]
fn receive_failure_reason_ok () {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    let props_panicker = Props::new(Arc::new(SimpleActor5::new), ());
    let props_handler = Props::new(Arc::new(SimpleActor6::new), tx);
    let panicker = actor_system.actor_of(props_panicker.clone(), "panicker".to_owned());
    let handler = actor_system.actor_of(props_handler.clone(), "handler".to_owned());

    panicker.tell_to(handler.clone(), SimpleActor6Messages::RegisterMe);

    let res = rx.recv();
    assert_eq!(Ok(Res::Ok), res);

    actor_system.shutdown();
}

#[test]
fn receive_failure_reason_err () {
    let actor_system = ActorSystem::new("test".to_owned());

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));

    // This actors does not fail with the "failure" reason, so the reason will not be correct
    let props_panicker = Props::new(Arc::new(SimpleActor3::new), ());
    let props_handler = Props::new(Arc::new(SimpleActor6::new), tx);
    let panicker = actor_system.actor_of(props_panicker.clone(), "panicker".to_owned());
    let handler = actor_system.actor_of(props_handler.clone(), "handler".to_owned());

    panicker.tell_to(handler.clone(), SimpleActor6Messages::RegisterMe);

    let res = rx.recv();
    assert_eq!(Ok(Res::Err), res);

    actor_system.shutdown();
}
