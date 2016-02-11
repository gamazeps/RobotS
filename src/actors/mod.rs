pub use std::any::Any;

pub use self::actor_cell::{ActorCell, ActorContext, ControlMessage, InnerMessage, SystemMessage};
pub use self::actor_ref::{ActorPath, ActorRef};
pub use self::actor_system::ActorSystem;
pub use self::props::Props;

/// Module for ActorRef and CanReceive, the interface given to the user to interract with  actors.
pub mod actor_ref;

/// Module for the ActorSystem.
pub mod actor_system;

/// Module for Actor factories (Props).
pub mod props;

/// Module with the internals of Actors.
pub mod actor_cell;

/// Module containing the original actor.
mod cthulhu;

/// Module containing the root actor pattern, used for the `user actor` and the `systel actor`.
mod root_actor;

/// Module with the name resolver actor.
mod name_resolver;

/// Robots Future implementation.
mod future;

/// Trait to be implemented by messages, this is automatically given if a struct is
/// already `Clone + Send + Sync + 'static + Any`.
pub trait Message: Clone + Send + Sync + 'static + Any {}
impl<T> Message for T where T: Clone + Send + Sync + 'static + Any
{}

/// Trait to be implemented by args, this is automatically given if a struct is
/// already `Clone + Send + Sync + 'static + Any`.
pub trait Arguments: Clone + Send + Sync + 'static {}
impl<T> Arguments for T where T: Clone + Send + Sync + 'static
{}

/// This is the trait to implement to become an Actor.
///
/// Normaly only the receive method has to be implemented.
pub trait Actor: Send + Sync + 'static {

    /// Single method to be implemented for an Actor.
    ///
    /// This defines the Actor's behaviour.
    // We have a Box<Any> in the API even though that is a Box<Message> in reality, this is
    // done in order to have nicer code for the downcasts (indeed, I can't implement downcast
    // methods for Box<Message>).
    // Checks for sending data with the Message trait is done in the sending phase.
    fn receive(&self, message: Box<Any>, context: ActorCell);

    /// Method called when a monitored actor is terminated.
    ///
    /// This is put in a separated method because match in rust must check all variations and we
    /// chose not to force the user to make a case for terminations if it does not monitor any
    /// actor.
    // NOTE: this currently panic! because termination notices are not sent in the current
    // implementation.
    fn receive_termination(&self, _context: ActorCell) {
        unimplemented!();
    }

    /// Method called before the Actor is started.
    fn pre_start(&self, _context: ActorCell) {}

    /// Method called after the Actor is stopped.
    fn post_stop(&self) {}

    /// Method called before the Actor is restarted.
    fn pre_restart(&self, _context: ActorCell) {
        self.post_stop();
    }

    /// Method called after the Actor is restarted.
    fn post_restart(&self, context: ActorCell) {
        self.pre_start(context);
    }
}
