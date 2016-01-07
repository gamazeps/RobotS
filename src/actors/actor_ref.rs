extern crate eventual;

use self::eventual::{Complete, Future};

use std::any::Any;
use std::sync::{Arc, Mutex};

use actors::{ActorContext, InnerMessage, Message, SystemMessage};
use actors::actor_cell::ActorCell;
use actors::cthulhu::Cthulhu;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum ActorPath {
    Local(String),
    Distant(ConnectionInfo),
}

impl ActorPath {
    pub fn new_local(path: String) -> Arc<ActorPath> {
        Arc::new(ActorPath::Local(path))
    }

    pub fn logical_path(&self) -> &String {
        match *self {
            ActorPath::Local(ref s) => s,
            ActorPath::Distant(ref c) => &(c.distant_logical_path),
        }
    }

    pub fn child(&self, name: String) -> Arc<ActorPath> {
        match *self {
            ActorPath::Local(ref s) => {
                let path = format!("{}/{}", s, name);
                ActorPath::new_local(path)
            },
            ActorPath::Distant(_) => panic!("Cannot create a child for a distant actor."),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ConnectionInfo {
    distant_logical_path: String,
    addr_port: String,
}

#[derive(Clone)]
enum InnerActor {
    Cthulhu(Cthulhu),
    Actor(ActorCell),
    Complete(CompleteRef),
}

#[derive(Clone)]
struct CompleteRef {
    complete: Arc<Mutex<Option<Complete<Box<Any + Send>, &'static str>>>>,
}

impl CompleteRef {
    fn new(complete: Complete<Box<Any + Send>, &'static str>) -> CompleteRef {
        CompleteRef {
            complete: Arc::new(Mutex::new(Some(complete))),
        }
    }

    fn complete(&self, message: InnerMessage) {
        match message {
            InnerMessage::Message(data) => {
                let mut guard = self.complete.lock().unwrap();
                let complete = guard.take();
                *guard = None;
                match complete {
                    Some(complete) => complete.complete(data),
                    None => println!("Tried to send more than one message to a Complete"),
                }
            },
            _ => panic!("Send a weird inner message to a future, this is a bug.")
        }
    }
}

pub struct ActorRef {
    inner_actor: Option<InnerActor>,
    path: Arc<ActorPath>,
}

impl ActorRef {
    pub fn new_distant(path: Arc<ActorPath>) -> ActorRef {
        ActorRef {
            inner_actor: None,
            path: path,
        }
    }

    pub fn with_cthulhu(cthulhu: Cthulhu) -> ActorRef {
        let path = ActorPath::new_local("/".to_owned());
        ActorRef {
            inner_actor: Some(InnerActor::Cthulhu(cthulhu)),
            path: path,
        }
    }

    pub fn with_cell(cell: ActorCell, path: Arc<ActorPath>) -> ActorRef {
        ActorRef {
            inner_actor: Some(InnerActor::Actor(cell)),
            path: path,
        }
    }

    pub fn with_complete(complete: Complete<Box<Any + Send>, &'static str>) -> ActorRef {
        ActorRef {
            inner_actor: Some(InnerActor::Complete(CompleteRef::new(complete))),
            // FIXME(gamazeps) future registration is not working here, this is not cool.
            path: ActorPath::new_local("local_future".to_owned()),
        }
    }

    pub fn receive_system_message(&self, system_message: SystemMessage) {
        let inner = self.inner_actor.as_ref().expect("Tried to put a system message in the mailbox of a distant actor.");
        match *inner {
            InnerActor::Complete(_) => panic!("Futures should not receive system messages."),
            InnerActor::Actor(ref actor) => actor.receive_system_message(system_message),
            InnerActor::Cthulhu(ref cthulhu) => cthulhu.receive_system_message(),
        };
    }

    pub fn receive(&self, message: InnerMessage, sender: ActorRef) {
        let inner = self.inner_actor.as_ref().expect("Tried to put a message in the mailbox of a distant actor.");
        match *inner {
            InnerActor::Complete(ref complete) => complete.complete(message),
            InnerActor::Actor(ref actor) => actor.receive_message(message, sender),
            InnerActor::Cthulhu(ref cthulhu) => cthulhu.receive(),
        };
    }

    pub fn handle(&self) {
        let inner = self.inner_actor.as_ref().expect("");
        match *inner {
            InnerActor::Complete(_) => panic!("In the current model futures should not handle messages."),
            InnerActor::Actor(ref actor) => actor.handle_envelope(),
            InnerActor::Cthulhu(ref cthulhu) => cthulhu.handle(),
        };
    }

    pub fn path(&self) -> Arc<ActorPath> {
        self.path.clone()
    }

    pub fn tell_to<MessageTo: Message>(&self, to: ActorRef, message: MessageTo) {
        let inner = self.inner_actor.as_ref().expect("");
        match *inner {
            InnerActor::Complete(_) => panic!("A future should not be sending a message to an actor this way."),
            InnerActor::Actor(_) => to.receive(InnerMessage::Message(Box::new(message) as Box<Any + Send>), self.clone()),
            InnerActor::Cthulhu(ref cthulhu) => cthulhu.receive(),
        };
    }

    pub fn ask<MessageTo: Message>(&self, message: MessageTo)
        -> Future<Box<Any + Send>, &'static str> {
        let (complete, future) = Future::<Box<Any + Send>, &'static str>::pair();
        self.receive(InnerMessage::Message(Box::new(message) as Box<Any + Send>),
                    ActorRef::with_complete(complete));
        future
    }
}

impl Clone for ActorRef {
    fn clone(&self) -> ActorRef {
        ActorRef {
            inner_actor: self.inner_actor.clone(),
            path: self.path.clone(),
        }
    }
}
