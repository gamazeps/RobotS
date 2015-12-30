use std::sync::Arc;

use actors::{ActorPath, ActorSystem, CanReceive, InnerMessage, SystemMessage};

/// Cthulhu is the original Actor in the Actor Hierarchy (used as the father of the root actor).
/// Naturaly waking Cthulhu up (by sending him a message) will wreck havoc on your application.
pub struct Cthulhu {
    path: ActorPath,
    actor_system: ActorSystem,
}

impl Cthulhu {
    /// Constructor.
    pub fn new(actor_system: ActorSystem) -> Cthulhu {
        Cthulhu {
            path: Arc::new("/".to_owned()),
            actor_system: actor_system,
        }
    }
}

impl CanReceive for Cthulhu {
    fn receive_system_message(&self, _system_message: SystemMessage) {
        self.actor_system.shutdown();
        panic!("Send a system message to the original actor.\r\n
                This should \
                never happen !");
    }

    fn receive(&self, _message: InnerMessage, _sender: Arc<CanReceive>) {
        self.actor_system.shutdown();
        panic!("Send a message to the original actor. \r\n
                This should happen \
                only if the root actor sends him a messages and that happens
                \
                only if he fails.");
    }

    fn handle(&self) {
        self.actor_system.shutdown();
        panic!("Asked the original actor to handle a message. \r\n
                This should \
                never happen.");
    }

    fn path(&self) -> ActorPath {
        self.path.clone()
    }
}
