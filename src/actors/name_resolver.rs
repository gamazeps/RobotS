use std::collections::HashMap;
use std::sync::Mutex;

use std::any::Any;
use std::sync::Arc;

use actors::{Actor, ActorCell, ActorContext, CanReceive};

/// Messages handled by the NameResolver.
#[derive(Clone)]
pub enum ResolveRequest {
    /// Used when we create an actor.
    Add(Arc<CanReceive>),

    /// Used when we delete an actor.
    Remove(Arc<String>),

    /// Used when we want to find the actor associated to a path.
    Get(String),
}

pub struct NameResolver {
    index: Mutex<HashMap<String, Arc<CanReceive>>>,
}

impl Actor for NameResolver {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<ResolveRequest>(message) {
            match *message {
                ResolveRequest::Add(address) => {
                    let mut index = self.index.lock().unwrap();
                    index.insert((*address.path()).clone(), address);
                }
                ResolveRequest::Remove(address) => {
                    let mut index = self.index.lock().unwrap();
                    index.remove(&*address);
                }
                ResolveRequest::Get(address) => {
                    let index = self.index.lock().unwrap();
                    context.tell(context.sender(), index.get(&address).cloned());
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
