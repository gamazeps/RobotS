use std::collections::VecDeque;
use std::sync::Arc;

use {Actor, ActorRef, Message, Props};

pub struct ActorCell<A: Actor> {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    inner_cell: Arc<InnerActorCell<A>>,
}

impl<A: Actor> Clone for ActorCell<A> {
    fn clone(&self) -> ActorCell<A> {
        ActorCell {
            inner_cell: self.inner_cell.clone()
        }
    }
}

impl<A: Actor> ActorCell<A> {
    pub fn new(actor: A, props: Props<A>) -> ActorCell<A> {
        ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props)),
        }
    }
}

/// This is the API that actors are supposed to see.
trait ActorContext<A: Actor> {
    fn actor_ref(&self) -> ActorRef<A>;
    fn actor_of(&self, props: Props<A>) -> ActorRef<A>;
}

impl<A: Actor> ActorContext<A> for ActorCell<A> {
    fn actor_ref(&self) -> ActorRef<A> {
        ActorRef::with_cell(self.clone())
    }

    fn actor_of(&self, props: Props<A>) -> ActorRef<A> {
        let actor = A::new();
        let actor_cell  = ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props)),
        };
        ActorRef::with_cell(actor_cell)
    }
}

struct InnerActorCell<A: Actor> {
    actor: A,
    mailbox: VecDeque<Message>,
    props: Props<A>,
}

impl<A: Actor> InnerActorCell<A> {
    fn new(actor: A, props: Props<A>) -> InnerActorCell<A> {
        InnerActorCell {
            actor: actor,
            mailbox: VecDeque::new(),
            props: props,
        }
    }
}
