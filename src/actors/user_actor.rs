use std::any::Any;
use std::sync::Arc;

use actors::{Actor, ActorCell, ActorContext, ActorPath, ActorRef, ActorSystem, Arguments,
             CanReceive, InnerMessage, Props, SystemMessage};
use actors::cthulhu::Cthulhu;

pub struct UserActorRef {
    actor_cell: ActorCell<(), InternalUserActor>,
    path: ActorPath,
}

impl UserActorRef {
    /// Creates a UserActor.
    pub fn new(system: ActorSystem, cthulhu: Arc<Cthulhu>) -> UserActorRef {
        let props = Props::new(Arc::new(InternalUserActor::new), ());
        let actor = props.create();
        let name = Arc::new("/user".to_owned());
        let actor_cell = ActorCell::new(actor, props, system, cthulhu, name.clone(), name.clone());
        UserActorRef {
            actor_cell: actor_cell,
            path: name.clone(),
        }
    }

    /// Creates an actor for the user.
    pub fn actor_of<Args: Arguments, A: Actor + 'static>(&self,
                                                         props: Props<Args, A>,
                                                         name: String)
                                                         -> Arc<ActorRef<Args, A>> {
        self.actor_cell.actor_of(props, name)
    }
}

impl Clone for UserActorRef {
    fn clone(&self) -> UserActorRef {
        UserActorRef {
            actor_cell: self.actor_cell.clone(),
            path: self.path.clone(),
        }
    }
}

// FIXME(gamazeps) this is a copy of the code in src/actor_ref.rs, this is bad.
impl CanReceive for UserActorRef {
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

struct InternalUserActor;

impl InternalUserActor {
    fn new(_dummy: ()) -> InternalUserActor {
        InternalUserActor
    }
}

impl Actor for InternalUserActor {
    // The receive function is currently a dummy.
    fn receive<Args: Arguments>(&self,
                                _message: Box<Any>,
                                _context: ActorCell<Args, InternalUserActor>) {
    }
}
