use std::any::Any;
use std::fmt::Debug;
use std::marker::Sized;

use actors::{Actor, ActorCell, ActorContext, Arguments};

struct DeadLetters;

impl Actor for DeadLetters {
    fn receive<Args: Arguments>(&self, message: Box<Any>, context: ActorCell<Args, DeadLetters>) {
        if let Ok(message) = Box::<Any>::downcast::<Debug + Sized>(message) {
            println!("received message: {}, from actor: {}", *message, context.sender().path());
        }
    }
}

impl DeadLetters {
    fn new(_dummy: ()) -> DeadLetters {
        DeadLetters
    }
}
