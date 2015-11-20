use std::collections::VecDeque;
use std::sync::Arc;

use {Actor, ActorRef, Message, Props};

pub struct ActorCell<Args: Copy, A: Actor> {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    inner_cell: Arc<InnerActorCell<Args, A>>,
}

impl<Args:  Copy, A: Actor> Clone for ActorCell<Args, A> {
    fn clone(&self) -> ActorCell<Args, A> {
        ActorCell {
            inner_cell: self.inner_cell.clone()
        }
    }
}

impl<Args: Copy, A: Actor> ActorCell<Args, A> {
    pub fn new(actor: A, props: Props<Args, A>) -> ActorCell<Args, A> {
        ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props)),
        }
    }
}

/// This is the API that actors are supposed to see.
trait ActorContext<Args: Copy, A: Actor> {
    fn actor_ref(&self) -> ActorRef<Args, A>;
    fn actor_of(&self, props: Props<Args, A>) -> ActorRef<Args, A>;
}

impl<Args: Copy, A: Actor> ActorContext<Args, A> for ActorCell<Args, A> {
    fn actor_ref(&self) -> ActorRef<Args, A> {
        ActorRef::with_cell(self.clone())
    }

    fn actor_of(&self, props: Props<Args, A>) -> ActorRef<Args, A> {
        let actor = props.create();
        let actor_cell  = ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props)),
        };
        ActorRef::with_cell(actor_cell)
    }
}

struct InnerActorCell<Args: Copy, A: Actor> {
    actor: A,
    mailbox: VecDeque<Message>,
    props: Props<Args, A>,
}

impl<Args: Copy, A: Actor> InnerActorCell<Args, A> {
    fn new(actor: A, props: Props<Args, A>) -> InnerActorCell<Args, A> {
        InnerActorCell {
            actor: actor,
            mailbox: VecDeque::new(),
            props: props,
        }
    }
}
