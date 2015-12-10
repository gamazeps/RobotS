use std::any::Any;
use std::sync::Arc;

use actors::{ActorPath, CanReceive, SystemMessage};

/// Cthulhu is the original Actor in the Actor Hierarchy (used as the father of the root actor).
/// Naturaly waking Cthulhu up (by sending him a message) will wreck havoc on your application.
pub struct Cthulhu {
    path: ActorPath,
}

impl Cthulhu {
    /// Constructor.
    pub fn new() -> Cthulhu {
        Cthulhu {
            path: Arc::new("/".to_owned()),
        }
    }
}

impl CanReceive for Cthulhu {
    fn receive_system_message(&self, _system_message: SystemMessage) {
        // Here we choose to exit the whole application as this is a hard error.
        // TODO(gamazeps): need to shut down the system in a clean way in this case.
        panic!("Send a system message to the original actor.\r\n
                 This should never happen !");
    }

    fn receive(&self, _message: Box<Any>, _sender: Arc<CanReceive >) {
        // TODO(gamazeps): need to shut down the system in a clean way in this case.
        panic!("Send a message to the original actor. \r\n
            This should happen only if the root actor sends him a messages and that happens only if
            he fails.");
    }

    fn handle(&self) {
        // TODO(gamazeps): need to shut down the system in a clean way in this case.
        panic!("Asked the original actor to handle a message. \r\n
            This should never happen.");
    }

    fn path(&self) -> ActorPath {
        self.path.clone()
    }
}
