use actors::ActorSystem;

/// Cthulhu is the original Actor in the Actor Hierarchy (used as the father of the root actor).
/// Naturaly waking Cthulhu up (by sending him a message) will wreck havoc on your application.
pub struct Cthulhu {
    actor_system: ActorSystem,
}

impl Cthulhu {
    /// Constructor.
    pub fn new(actor_system: ActorSystem) -> Cthulhu {
        Cthulhu {
            actor_system: actor_system,
        }
    }

    pub fn receive_system_message(&self) -> ! {
        self.actor_system.shutdown();
        panic!("Send a system message to the original actor.\r\n
                This should \
                never happen !")
    }

    pub fn receive(&self) -> ! {
        self.actor_system.shutdown();
        panic!("Send a message to the original actor. \r\n
                This should happen \
                only if the root actor sends him a messages and that happens
                \
                only if he fails.")
    }

    pub fn handle(&self) -> ! {
        self.actor_system.shutdown();
        panic!("Asked the original actor to handle a message. \r\n
                This should \
                never happen.")
    }
}

impl Clone for Cthulhu {
    fn clone(&self) -> Cthulhu {
        Cthulhu {
            actor_system: self.actor_system.clone()
        }
    }
}
