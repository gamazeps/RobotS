use std::any::Any;

use ActorCell;

/// This is the trait to implement to become an Actor.
///
/// Normaly only the receive method has to be implemented.
pub trait Actor<M: Copy + Send + Sync + 'static + Any>: Send  + Sized {

    /// Single method to be implemented for an Actor.
    ///
    /// This defines the Actor's behaviour.
    ///
    /// extern crate robots;
    /// use robots::{Actor, ActorCell, Message};
    ///
    /// struct MyActor;
    ///
    /// impl Actor for MyActor {
    ///     fn receive<Args: Copy + Send + Sync + 'static>
    ///         (&self, message: Message, _context: ActorCell<Args, MyActor>) {
    ///         match message {
    ///             Message::Dummy => context.tell(context.sender(), Message::Dummy),
    ///             _ => println!("Hi !"),
    ///         }
    ///     }
    /// }
    fn receive<Args: Copy + Send + Sync + 'static>(&self, message: M, context: ActorCell<Args, M, Self>);

    /// Method called before the Actor is started.
    fn pre_start(&self) {
        panic!("Not implemented");
    }

    /// Method called after the Actor is stopped.
    fn post_stop(&self) {
        panic!("Not implemented");
    }

    /// Method called before the Actor is restarted.
    fn pre_restart(&self) {
        panic!("Not implemented");
    }

    /// Method called after the Actor is restarted.
    fn post_restart(&self) {
        panic!("Not implemented");
    }
}

