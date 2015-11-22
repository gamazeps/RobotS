use {Actor, Message};
use actor_cell::ActorCell;

pub struct ActorRef<Args: Copy, A: Actor> {
    actor_cell: ActorCell<Args, A>,
}

impl<Args: Copy, A: Actor> Clone for ActorRef<Args, A> {
    fn clone(&self) -> ActorRef<Args, A> {
        ActorRef::with_cell(self.actor_cell.clone())
    }
}

impl<Args: Copy, A: Actor> ActorRef<Args, A> {
    pub fn with_cell(cell: ActorCell<Args, A>) -> ActorRef<Args, A> {
        ActorRef {
            actor_cell: cell,
        }
    }
}

/// Trait used to signal that a struct can receive messages.
/// Note that for the moment these are not typed, but it will be easy to add.
pub trait CanReceive {
    fn receive(&self, message: Message);
    fn handle(&self);
}

impl<Args: Copy, A: Actor> CanReceive for ActorRef<Args, A> {
    fn receive(&self, message: Message) {
        self.actor_cell.receive_message(message);
    }

    fn handle(&self) {
        self.actor_cell.handle_envelope();
    }
}
