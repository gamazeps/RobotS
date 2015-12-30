pub use std::any::Any;

pub use self::actor_cell::{ActorCell, ActorContext, ControlMessage, InnerMessage, SystemMessage};
pub use self::actor_ref::{CanReceive, ActorPath, ActorRef};
pub use self::actor_system::ActorSystem;
pub use self::props::Props;

/// Module for ActorRef, what is used for manipulating Actors.
pub mod actor_ref;

/// Module for the ActorSystem.
pub mod actor_system;

/// Module for Actor factories (Props).
pub mod props;

/// Module with the internals of Actors.
pub mod actor_cell;

/// Module allowing the use of the ask pattern.
pub mod ask;

/// Module containing the original actor.
mod cthulhu;

/// Module containing the user actor, parent of all user generated actors.
mod user_actor;

/// Module containing the system actor, parent of all actor system generated actors.
mod system_actor;

/// Trait to be implemented by messages, this is automatically given if a struct is
/// already `Copy + Send + Sync + 'static + Any`.
pub trait Message: Copy + Send + Sync + 'static + Any {}
impl<T> Message for T where T: Copy + Send + Sync + 'static + Any
{}

/// Trait to be implemented by args, this is automatically given if a struct is
/// already `Clone + Sync + 'static + Any`.
pub trait Arguments: Clone + Send + Sync + 'static {}
impl<T> Arguments for T where T: Clone + Send + Sync + 'static
{}

/// This is the trait to implement to become an Actor.
///
/// Normaly only the receive method has to be implemented.
pub trait Actor: Send + Sync + Sized {

    /// Single method to be implemented for an Actor.
    ///
    /// This defines the Actor's behaviour.
    // NOTE: we have a Box<Any> in the API even though that is a Box<Message> in reality, this is
    // done in order to have nicer code for the downcasts (indeed, I can't implement downcast
    // methods for Box<Message>).
    // Checks for sending data with the Message trait is done in the sending phase.
    fn receive<Args: Arguments>(&self, message: Box<Any>, context: ActorCell<Args, Self>);

    /// Method called when a monitored actor is terminated.
    ///
    /// This is put in a separated method because match in rust must check all variations and we
    /// chose not to force the user to make a case for terminations if it doesn not monitor any
    /// actor.
    fn receive_termination<Args: Arguments>(&self, _context: ActorCell<Args, Self>) {
        panic!("Not implemented");
    }

    /// Method called before the Actor is started.
    fn pre_start<Args: Arguments>(&self, _context: ActorCell<Args, Self>) {}

    /// Method called after the Actor is stopped.
    fn post_stop(&self) {}

    /// Method called before the Actor is restarted.
    fn pre_restart<Args: Arguments>(&self, _context: ActorCell<Args, Self>) {
        self.post_stop();
    }

    /// Method called after the Actor is restarted.
    fn post_restart<Args: Arguments>(&self, context: ActorCell<Args, Self>) {
        self.pre_start(context);
    }
}
