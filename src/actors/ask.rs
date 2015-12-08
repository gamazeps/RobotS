extern crate eventual;

use std::any::Any;
use std::sync::{Arc, Mutex};

use self::eventual::{Future, Complete};

use actors::{Actor, ActorCell, ActorContext, ActorPath, CanReceive, Message, SystemMessage};

struct CompleteRef<T: Send + 'static, E: Send + 'static> {
    complete: Mutex<Option<Complete<T, E>>>,
    path: ActorPath,
}

impl<M: Message,
    E: Send + 'static>
    CanReceive for CompleteRef<M, E> {
    fn receive(&self, message: Box<Any>, _: Arc<CanReceive >) {
        let cast = message.downcast::<M>();
        match cast {
            Ok(message) => {
                let mut guard = self.complete.lock().unwrap();
                let complete = guard.take();
                *guard = None;
                match complete {
                    Some(complete) => {
                        complete.complete(*message);
                    },
                    None => {
                        println!("Tried to send more than one message to a Complete");
                    },
                }
            },
            Err(_) => {
                println!("Send a message of the wrong type to a future");
            },
        }
    }

    fn receive_system_message(&self, _: SystemMessage) {
        println!("Tried to send a SystemMessage to a Complete");
    }

    fn handle(&self) {
    }

    fn path(&self) -> ActorPath {
        self.path.clone()
    }
}

/// Trait to implement for having the ask method.
pub trait AskPattern<Args: Message, M: Message, A: Actor<M> + 'static, E: Send + 'static>: ActorContext<Args, M, A> {
    /// Sends a request to an Actor and stores the potential result in a Future.
    ///
    /// The Future will be completed with the value the actor will answer with.
    fn ask<Message: Copy + Send + 'static + Any, T: CanReceive>(&self, to: T, message: Message)
        -> Future<M, E>;
}

impl<Args: Message,
M: Message,
A: Actor<M> + 'static,
E: Send + 'static>
AskPattern<Args, M, A, E> for ActorCell<Args, M, A> {
    fn ask<Message: Copy + Send + 'static + Any, T: CanReceive>(&self, to: T, message: Message) -> Future<M, E> {
        let (complete, future) = Future::<M, E>::pair();
        let complete_ref = CompleteRef {
            complete: Mutex::new(Some(complete)),
            path: Arc::new("".to_owned()),
        };
        to.receive(Box::new(message), Arc::new(complete_ref));
        future
    }
}
