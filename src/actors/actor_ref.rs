extern crate eventual;

use self::eventual::Future;

use std::sync::Arc;

use actors::{ActorContext, InnerMessage, Message, SystemMessage};
use actors::actor_cell::ActorCell;
use actors::ask::AskPattern;

/// Type used to represent an ActorPath.
/// This is juste an Arc<String> now but will contain more information later.
pub type ActorPath = Arc<String>;

/// This is a reference to an Actor and what is supposed to be manipulated by the user.
///
/// The only thing it can do is send and receive messages (according to the actor model defined
/// by Hewitt).
pub struct ActorRef {
    actor_cell: ActorCell,
    path: ActorPath,
}

impl Clone for ActorRef {
    fn clone(&self) -> ActorRef {
        ActorRef::with_cell(self.actor_cell.clone(), self.path().clone())
    }
}

impl ActorRef {
    /// Creates an ActorRef with the given ActorCell.
    pub fn with_cell(cell: ActorCell, path: ActorPath) -> ActorRef {
        ActorRef {
            actor_cell: cell,
            path: path,
        }
    }

    /// Sends a Message to a CanReceive.
    pub fn tell_to<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo) {
        self.actor_cell.tell(to, message);
    }

    /// Sends a request to a CanReceive, the answer to this request is put in the Future returned
    /// by ask_to.
    pub fn ask_to<MessageTo: Message, V: Message, E: Send + 'static>(&self,
                                                                     to: Arc<CanReceive>,
                                                                     message: MessageTo)
                                                                     -> Future<V, E> {
        self.actor_cell.ask(to, message)
    }
}

/// This is the interface used for structs to which we can send messages.
///
/// We may call them ActorRef in the documentation because they act as an interface to something
/// that acts like an actor (abd are actually ActorRef most of the time)..
pub trait CanReceive: Send + Sync {

    /// Receives a message and then schedules the CanReceive in the ActorSystem for handling the
    /// message.
    fn receive(&self, message: InnerMessage, sender: Arc<CanReceive>);

    /// Receives a system generated message and then schedules the CanReceive in the ActorSystem for
    /// handling the message.
    ///
    /// This is a diffreen method than receive_message because it does not require sender
    /// information.
    fn receive_system_message(&self, system_message: SystemMessage);

    /// Handles a message that has been received.
    fn handle(&self);

    /// Logical path to the CanReceive.
    fn path(&self) -> ActorPath;

    /// Cheks if two CanReceive have the same underlying actor (or some other structure).
    ///
    /// Note that it only compares the logical path.
    // FIXME(gamazeps): investigate having an ID for actors, so that we can check if two ActorRef
    // with the same logical path really have the same underlying actor.
    // This could happpen if I hold an old reference to an actor with some path, this actor is
    // deleted and then a new actor is created with the same path.
    fn equals(&self, other: &CanReceive) -> bool {
        self.path() == other.path()
    }
}

impl CanReceive for ActorRef {
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
