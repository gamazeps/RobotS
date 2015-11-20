use {Actor, Message};
use actor_cell::ActorCell;

pub struct ActorRef<A: Actor> {
    actor_cell: ActorCell<A>,
}

impl<A: Actor> ActorRef<A> {
    pub fn with_cell(cell: ActorCell<A>) -> ActorRef<A> {
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


impl<A: Actor> CanSend for ActorRef<A> {
    fn send(&self, message: Message) {}
}

impl<A: Actor> CanReceive for ActorRef<A> {}

struct ActorPath {
    path: String,
}
