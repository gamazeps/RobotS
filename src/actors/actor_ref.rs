extern crate eventual;

use self::eventual::Future;

use std::any::Any;
use std::sync::Arc;

use actors::{Actor, ActorContext, Arguments, ControlMessage, InnerMessage, Message, SystemMessage};
use actors::actor_cell::ActorCell;
use actors::ask::AskPattern;

/// Type used to represent an ActorPath.
/// This is juste a string now but will contain more information later.
pub type ActorPath = Arc<String>;

/// This is a reference to an Actor and what is supposed to be manipulated by the user.
///
/// The only thing it can do is send and receive messages (according to the actor model in defined
/// by Hewitt).
pub struct ActorRef<Args: Arguments, M: Message, A: Actor<M> + 'static> {
    actor_cell: ActorCell<Args, M, A>,
    path: ActorPath,
}

impl<Args: Arguments, M: Message, A: Actor<M> + 'static> Clone for ActorRef<Args, M, A> {
    fn clone(&self) -> ActorRef<Args, M, A> {
        ActorRef::with_cell(self.actor_cell.clone(), self.path().clone())
    }
}

impl<Args: Arguments, M: Message, A: Actor<M> + 'static> ActorRef<Args, M, A> {
    /// Creates an ActorRef<Args, M, A> with the given ActorCell<Args, M, A>.
    pub fn with_cell(cell: ActorCell<Args, M, A>, path: ActorPath) -> ActorRef<Args, M, A> {
        ActorRef {
            actor_cell: cell,
            path: path,
        }
    }

    /// Sends a Message to a CanReceive<Message>.
    pub fn tell_to<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo) {
        self.actor_cell.tell(to, message);
    }

    /// Sends a Message to a CanReceive<Message>.
    pub fn ask_to<MessageTo: Message, V: Message, E: Send + 'static>(&self,
                                                                     to: Arc<CanReceive>,
                                                                     message: MessageTo)
                                                                     -> Future<V, E> {
        self.actor_cell.ask(to, message)
    }
}

/// Trait used to signal that a struct can receive messages.
/// Note that for the moment these are not typed, but it will be easy to add.
pub trait CanReceive: Send + Sync {
    /// Puts the message in a mailbox and enqueues the CanReceive.
    fn receive(&self, message: Box<Any>, sender: Arc<CanReceive>);

    /// Puts the system message in a mailbox and enqueues the CanReceive.
    fn receive_system_message(&self, system_message: SystemMessage);

    /// Handles the message.
    ///
    /// Thus completes a Promise or calls the Actor's receive method.
    fn handle(&self);

    /// Path to the actor.
    fn path(&self) -> ActorPath;

    /// Tool for comparing actor refs.
    fn equals(&self, other: &CanReceive) -> bool {
        self.path() == other.path()
    }
}

impl<Args: Arguments, M: Message, A: Actor<M> + 'static> CanReceive for ActorRef<Args, M, A> {
    fn receive(&self, message: Box<Any>, sender: Arc<CanReceive>) {
        match message.downcast::<ControlMessage>() {
            Ok(message) => {
                self.actor_cell.receive_message(InnerMessage::Control(*message), sender);
                return;
            }
            Err(message) => {
                match message.downcast::<M>() {
                    Ok(message) => {
                        self.actor_cell.receive_message(InnerMessage::Message(*message), sender)
                    }
                    Err(_) => {
                        println!("Send a message of the wrong type to an actor");
                    }
                }
            }
        }
    }

    fn receive_system_message(&self, system_message: SystemMessage) {
        self.actor_cell.receive_system_message(system_message);
    }

    fn handle(&self) {
        self.actor_cell.handle_envelope();
    }

    fn path(&self) -> ActorPath {
        self.path.clone()
    }
}
