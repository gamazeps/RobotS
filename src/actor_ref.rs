use std::sync::Arc;
use std::any::Any;
use std::marker::Reflect;

use Actor;
use actor_cell::{ActorCell, ActorContext};

/// This is a reference to an Actor and what is supposed to be manipulated by the user.
///
/// The only thing it can do is send and receive messages (according to the actor model in defined
/// by Hewitt).
pub struct ActorRef<Args: Copy + Sync + Send + 'static, M: Copy + Sync + Send + 'static + Reflect, A: Actor<M> + 'static> {
    actor_cell: ActorCell<Args, M, A>,
}

impl<Args: Copy + Sync + Send + 'static, M: Copy + Sync + Send + 'static + Reflect, A: Actor<M> + 'static> Clone for ActorRef<Args, M, A> {
    fn clone(&self) -> ActorRef<Args, M, A> {
        ActorRef::with_cell(self.actor_cell.clone())
    }
}

impl<Args: Copy + Sync + Send + 'static, M: Copy + Sync + Send + 'static + Reflect, A: Actor<M> + 'static> ActorRef<Args, M, A> {
    /// Creates an ActorRef<Args, M, A> with the given ActorCell<Args, M, A>.
    pub fn with_cell(cell: ActorCell<Args, M, A>) -> ActorRef<Args, M, A> {
        ActorRef {
            actor_cell: cell,
        }
    }

    /// Sends a Message to a CanReceive<Message>.
    pub fn tell_to<Message: Copy + Sync + Send + 'static + Reflect, T: CanReceive>(&self, to: T, message: Message) {
        self.actor_cell.tell(to, message);
    }
}

/// Trait used to signal that a struct can receive messages.
/// Note that for the moment these are not typed, but it will be easy to add.
pub trait CanReceive: Send + Sync {
    /// Puts the message in a mailbox and enqueues the CanReceive.
    fn receive(&self, message: Box<Any>, sender: Arc<CanReceive + Sync>);

    /// Handles the message.
    ///
    /// Thus completes a Promise or calls the Actor's receive method.
    fn handle(&self);
}

impl<Args: Copy + Sync + Send + 'static, M: Copy + Sync + Send + 'static + Reflect, A: Actor<M> + 'static> CanReceive for ActorRef<Args, M, A> {
    fn receive(&self, message: Box<Any>, sender: Arc<CanReceive + Sync>) {
        let cast = message.downcast::<M>();
        match cast {
            Ok(message) => self.actor_cell.receive_message(*message, sender),
            Err(_) => panic!("Send a message of the wrong type to an actor"),
        }
    }

    fn handle(&self) {
        self.actor_cell.handle_envelope();
    }
}
