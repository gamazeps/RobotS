extern crate eventual;

use self::eventual::Future;

use std::sync::Arc;

use actors::{Actor, ActorContext, Arguments, InnerMessage, Message, SystemMessage};
use actors::actor_cell::ActorCell;
use actors::ask::AskPattern;

/// Type used to represent an ActorPath.
/// This is juste a string now but will contain more information later.
pub type ActorPath = Arc<String>;

/// This is a reference to an Actor and what is supposed to be manipulated by the user.
///
/// The only thing it can do is send and receive messages (according to the actor model in defined
/// by Hewitt).
pub struct ActorRef<Args: Arguments, A: Actor + 'static> {
    actor_cell: ActorCell<Args, A>,
    path: ActorPath,
}

impl<Args: Arguments, A: Actor + 'static> Clone for ActorRef<Args, A> {
    fn clone(&self) -> ActorRef<Args, A> {
        ActorRef::with_cell(self.actor_cell.clone(), self.path().clone())
    }
}

impl<Args: Arguments, A: Actor + 'static> ActorRef<Args, A> {
    /// Creates an ActorRef<Args, A> with the given ActorCell<Args, A>.
    pub fn with_cell(cell: ActorCell<Args, A>, path: ActorPath) -> ActorRef<Args, A> {
        ActorRef {
            actor_cell: cell,
            path: path,
        }
    }

    /// Sends a Message to a CanReceive<Message>.
    // Note that we do not need this to be generic and could be a Box<Any>, but it seems like a
    // nicer API to use.
    pub fn tell_to<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo) {
        self.actor_cell.tell(to, message);
    }

    /// Sends a Message to a CanReceive<Message>.
    // Note that we do not need this to be generic and could be a Box<Any>, but it seems like a
    // nicer API to use.
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
    fn receive(&self, message: InnerMessage, sender: Arc<CanReceive>);

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

impl<Args: Arguments, A: Actor + 'static> CanReceive for ActorRef<Args, A> {
    fn receive(&self, message: InnerMessage, sender: Arc<CanReceive>) {
        self.actor_cell.receive_message(message, sender);
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
