// FIXME(gamazeps) this is a copy of the code in user_actor (with sed '/user/system/'), this is
// super bad.
use std::any::Any;
use std::sync::Arc;

use actors::{Actor, ActorCell, ActorContext, ActorPath, ActorRef, ActorSystem, Arguments,
             CanReceive, InnerMessage, Props, SystemMessage};
use actors::cthulhu::Cthulhu;

pub struct SystemActorRef {
    actor_cell: ActorCell<(), InternalSystemActor>,
    path: ActorPath,
}

impl SystemActorRef {
    /// Creates a SystemActor.
    pub fn new(system: ActorSystem, cthulhu: Arc<Cthulhu>) -> SystemActorRef {
        let props = Props::new(Arc::new(InternalSystemActor::new), ());
        let actor = props.create();
        let name = Arc::new("/system".to_owned());
        let actor_cell = ActorCell::new(actor, props, system, cthulhu, name.clone(), name.clone());
        SystemActorRef {
            actor_cell: actor_cell,
            path: name.clone(),
        }
    }

    #[allow(dead_code)]
    /// Creates an actor for the system.
    pub fn actor_of<Args: Arguments, A: Actor + 'static>(&self,
                                                         props: Props<Args, A>,
                                                         name: String)
                                                         -> Arc<ActorRef<Args, A>> {
        self.actor_cell.actor_of(props, name)
    }
}

impl Clone for SystemActorRef {
    fn clone(&self) -> SystemActorRef {
        SystemActorRef {
            actor_cell: self.actor_cell.clone(),
            path: self.path.clone(),
        }
    }
}

// FIXME(gamazeps) this is a copy of the code in src/actor_ref.rs, this is bad.
impl CanReceive for SystemActorRef {
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

struct InternalSystemActor;

impl InternalSystemActor {
    fn new(_dummy: ()) -> InternalSystemActor {
        InternalSystemActor
    }
}

impl Actor for InternalSystemActor {
    // The receive function is currently a dummy.
    fn receive<Args: Arguments>(&self,
                                _message: Box<Any>,
                                _context: ActorCell<Args, InternalSystemActor>) {
    }
}
