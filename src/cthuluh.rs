use std::any::Any;
use std::process::exit;
use std::sync::Arc;

use CanReceive;

/// Cthuluh is the original Actor in the Actor Hierarchy (used as the father of the root actor).
/// Naturaly waking Cthuluh up (by sending him a message) will wreck havoc on your application.
#[derive(Copy, Clone)]
pub struct Cthuluh;

impl Cthuluh {
    /// Constructor.
    pub fn new() -> Cthuluh {
        Cthuluh
    }
}

impl CanReceive for Cthuluh {
    fn receive(&self, _message: Box<Any>, _sender: Arc<CanReceive >) {
        println!("Send a message to the original actor. \r\n
            This should happen only if the root actor sends him a messages and that happens only if
            he fails.");
        // Here we choose to exit the whole application as this is a hard error.
        // TODO(gamazeps) define error codes for the application when more exit reason appear.
        exit(1);
    }

    fn handle(&self) {
        println!("Asked the original actor to handle a message. \r\n
            This should never happen.");
        // Here we choose to exit the whole application as this is a hard error.
        // TODO(gamazeps) define error codes for the application when more exit reason appear.
        exit(1);
    }
}
