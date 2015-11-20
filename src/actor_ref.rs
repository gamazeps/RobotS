use {Actor, Message};
use actor_cell::ActorCell;

pub struct ActorRef<Args: Copy, A: Actor> {
    actor_cell: ActorCell<Args, A>,
}

impl<Args: Copy, A: Actor> ActorRef<Args, A> {
    pub fn with_cell(cell: ActorCell<Args, A>) -> ActorRef<Args, A> {
        ActorRef {
            actor_cell: cell,
        }
    }
}

/// Trait used to signal that a struct can send messages.
trait CanSend {
    fn send(&self, message: Message);
}

/// Trait used to signal that a struct can receive messages.
/// This is used so that we can have future receive messages.
trait CanReceive {
}


impl<Args: Copy, A: Actor> CanSend for ActorRef<Args, A> {
    fn send(&self, message: Message) {}
}

impl<Args: Copy, A: Actor> CanReceive for ActorRef<Args, A> {}

struct ActorPath {
    path: String,
}
