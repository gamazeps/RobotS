use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actors::{Actor, ActorCell, ActorContext, ActorPath, ActorRef};

/// Messages handled by the NameResolver.
#[derive(Clone)]
pub enum ResolveRequest {
    /// Used when we create an actor.
    Add(ActorRef),

    /// Used when we delete an actor.
    Remove(Arc<ActorPath>),

    /// Used when we want to find the actor associated to a path.
    Get(String),
}

/// Name resolving actor.
///
/// It is used to resolve logical path to a real CanReceive.
///
/// It accepts ResolveRequest as messages.
///
/// When an actor is created its father sends a registration request to the anme resolver it.
/// When an actor terminates one of its children it send an unregistration request to the name
/// resolver.
pub struct NameResolver {
    index: Mutex<HashMap<Arc<ActorPath>, ActorRef>>,
}

impl Actor for NameResolver {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<ResolveRequest>(message) {
            match *message {
                ResolveRequest::Add(address) => {
                    let mut index = self.index.lock().unwrap();
                    index.insert(address.path(), address);
                }
                ResolveRequest::Remove(address) => {
                    let mut index = self.index.lock().unwrap();
                    index.remove(&address);
                }
                ResolveRequest::Get(address) => {
                    let index = self.index.lock().unwrap();
                    context.tell(context.sender(), index.get(&ActorPath::new_local(address)).cloned());
                }
            }
        }
    }
}

impl NameResolver {
    pub fn new(_dummy: ()) -> NameResolver {
        NameResolver { index: Mutex::new(HashMap::new()) }
    }
}
