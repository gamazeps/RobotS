use std::any::Any;
use std::sync::{Arc, Mutex};

use eventual::{Future, Complete};

use {Actor, ActorCell, ActorContext, CanReceive, SystemMessage};

impl<M: Copy + Send + Sync + 'static + Any,
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
                    None => panic!("Tried to send more than one message to a Complete"),
                }
            },
            Err(_) => panic!("Send a message of the wrong type to a future"),
        }
    }

    fn receive_system_message(&self, _: SystemMessage) {
        panic!("Tried to send a SystemMessage to an Actor");
    }

    fn handle(&self) {
    }
}

struct CompleteRef<T: Send + 'static, E: Send + 'static> {
    complete: Mutex<Option<Complete<T, E>>>,
}

/// Trait to implement for having the ask method.
pub trait AskPattern<Args: Copy + Send + Sync + 'static,
    M: Copy + Send + Sync + 'static + Any,
    A: Actor<M> + 'static,
    E: Send + 'static>: ActorContext<Args, M, A> {
    /// Sends a request to an Actor and stores the potential result in a Future.
    ///
    /// The Future will be completed with the value the actor will answer with.
    fn ask<Message: Copy + Send + 'static + Any, T: CanReceive>(&self, to: T, message: Message)
        -> Future<M, E>;
}

impl<Args: Copy + Send + Sync + 'static,
M: Copy + Send + Sync + 'static + Any,
A: Actor<M> + 'static,
E: Send + 'static>
AskPattern<Args, M, A, E> for ActorCell<Args, M, A> {
    fn ask<Message: Copy + Send + 'static + Any, T: CanReceive>(&self, to: T, message: Message) -> Future<M, E> {
        let (complete, future) = Future::<M, E>::pair();
        let complete_ref = CompleteRef { complete: Mutex::new(Some(complete)) };
        to.receive(Box::new(message), Arc::new(complete_ref));
        future
    }
}
