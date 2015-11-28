use std::any::Any;
use std::sync::Arc;

use {Actor, ActorCell, ActorContext, ActorRef, ActorSystem, CanReceive, Props, SystemMessage};
use cthulhu::Cthulhu;

pub struct UserActorRef {
    actor_cell: ActorCell<(), (), InternalUserActor>,
}

impl UserActorRef {
    /// Creates a UserActor.
    pub fn new(system: ActorSystem, cthulhu: Arc<Cthulhu>) -> UserActorRef {
        let props = Props::new(Arc::new(InternalUserActor::new), ());
        let actor = props.create();
        let actor_cell = ActorCell::new(actor, props, system, cthulhu);
        UserActorRef { actor_cell: actor_cell }
    }

    /// Creates an actor for the user.
    pub fn actor_of<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static>(&self, props: Props<Args, M, A>) -> ActorRef<Args, M, A> {
        self.actor_cell.actor_of(props)
    }
}

impl Clone for UserActorRef {
    fn clone(&self) -> UserActorRef {
        UserActorRef { actor_cell: self.actor_cell.clone() }
    }
}

impl CanReceive for UserActorRef {
    // TODO(gamazeps) this is a copy of the code in src/actor_ref.rs, this is bad.
    fn receive(&self, message: Box<Any>, sender: Arc<CanReceive >) {
        let cast = message.downcast::<()>();
        match cast {
            Ok(message) => self.actor_cell.receive_message(*message, sender),
            Err(_) => panic!("Send a message of the wrong type to an actor"),
        }
    }

    fn receive_system_message(&self, system_message: SystemMessage) {
        self.actor_cell.receive_system_message(system_message);
    }

    fn handle(&self) {
        self.actor_cell.handle_envelope();
    }
}

struct InternalUserActor;

impl InternalUserActor {
    fn new(_dummy: ()) -> InternalUserActor {
        InternalUserActor
    }
}

impl Actor<()> for InternalUserActor {
    // The recieve function is currently a dummy.
    fn receive<Args: Copy + Sync + Send + 'static>(&self, _message: (), _context: ActorCell<Args, (), InternalUserActor>) {}
}
