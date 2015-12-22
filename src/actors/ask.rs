extern crate eventual;

use std::any::Any;
use std::sync::{Arc, Mutex};

use self::eventual::{Complete, Future};

use actors::{Actor, ActorCell, ActorContext, ActorPath, Arguments, CanReceive, Message,
             SystemMessage};

struct CompleteRef<V: Send + 'static, E: Send + 'static> {
    complete: Mutex<Option<Complete<V, E>>>,
    path: ActorPath,
}

impl<V: Message, E: Send + 'static> CanReceive for CompleteRef<V, E> {
    fn receive(&self, message: Box<Any>, _: Arc<CanReceive>) {
        let cast = message.downcast::<V>();
        match cast {
            Ok(message) => {
                let mut guard = self.complete.lock().unwrap();
                let complete = guard.take();
                *guard = None;
                match complete {
                    Some(complete) => {
                        complete.complete(*message);
                    }
                    None => {
                        println!("Tried to send more than one message to a Complete");
                    }
                }
            }
            Err(_) => {
                println!("Send a message of the wrong type to a future");
            }
        }
    }

    fn receive_system_message(&self, _: SystemMessage) {
        println!("Tried to send a SystemMessage to a Complete");
    }

    fn handle(&self) {}

    fn path(&self) -> ActorPath {
        self.path.clone()
    }
}

/// Trait to implement for having the ask method.
pub trait AskPattern<Args, M, A, V, E>: ActorContext<Args, M, A>
where Args: Arguments,
      M: Message,
      A: Actor<M> + 'static,
      V: Message,
      E: Send + 'static
{
    /// Sends a request to an Actor and stores the potential result in a Future.
    ///
    /// The Future will be completed with the value the actor will answer with.
    fn ask<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo) -> Future<V, E>;
}

impl<Args, M, A, V, E> AskPattern<Args, M, A, V, E> for ActorCell<Args, M, A>
    where Args: Arguments,
          M: Message,
          A: Actor<M> + 'static,
          V: Message,
          E: Send + 'static
{
    fn ask<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo) -> Future<V, E> {
        let (complete, future) = Future::<V, E>::pair();
        let complete_ref = CompleteRef {
            complete: Mutex::new(Some(complete)),
            path: Arc::new("".to_owned()),
        };
        to.receive(Box::new(message), Arc::new(complete_ref));
        future
    }
}
