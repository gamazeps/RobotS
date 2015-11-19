use std::collections::VecDeque;

use {Actor, Message};

pub struct ActorCell<A: Actor> {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    cell_inner: InnerActorCell<A>
}

struct InnerActorCell<A: Actor> {
    actor: A,
    // TODO(gamazeps) deal with messages.
    mailbox: VecDeque<Message>,
}
