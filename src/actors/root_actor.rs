/// This is a template for root actors.
///
/// We qualify a `root actor` as the root of an actor hierarchy.
///
/// Note that there are only two root actors in the current implementation:
///
///   * The `user actor` which is the root for all user created actors.
///   * The `system actor` which is the root for all actors created for the actor system.
///
/// The father of these two actors is Cthulhu.

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use actors::{Actor, ActorCell, ActorContext, ActorRef};
use actors::props::ActorFactory;

pub struct RootActor;

impl RootActor {
    pub fn new(_dummy: ()) -> RootActor {
        RootActor
    }
}

impl Actor for RootActor {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<(Arc<ActorFactory>, String, Arc<Mutex<Sender<ActorRef>>>)>(message) {
            let tmp = *message;
            let (props, name, tx) = tmp;
            let actor_ref = context.actor_of(props, name);
            // FIXME(gamazeps): error handling.
            let _res = tx.lock().unwrap().send(actor_ref);
        }
    }
}
