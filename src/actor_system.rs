use {Actor, ActorRef, Props};
use actor_cell::ActorCell;

pub struct ActorSystem {
    name: String,
}

impl ActorSystem {
    pub fn new(name: String) -> ActorSystem {
        ActorSystem {
            name: name
        }
    }

    pub fn actor_of<Args: Copy, A: Actor>(&self, props: Props<Args, A>) -> ActorRef<Args, A> {
        let actor = props.create();
        let actor_cell = ActorCell::new(actor, props);
        ActorRef::with_cell(actor_cell)
    }
}
