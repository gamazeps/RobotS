use std::any::Any;

use ActorCell;

/// This is the trait to implement to become an Actor.
///
/// Normaly only the receive method has to be implemented.
pub trait Actor<M: Copy + Send + Sync + 'static + Any>: Send +  Sync + Sized {

    /// Single method to be implemented for an Actor.
    ///
    /// This defines the Actor's behaviour.
    ///
    /// Note that Actors have to be both Send AND Sync:
    ///   - The Send is obvious as they go around threads.
    ///   - The Sync is needed because of Actor failure handling.  
    ///   Actors need to be stored in InnerActorCell in a container that offers inner mutability
    ///   because of the ability to restart them (we replace the old failed actor with a new clean
    ///   one).  
    ///   Two container offer that in Sync way, `RwLock` and `Mutex`. `Mutex` would give Sync to
    ///   the Actor, but if a thread fails while holding a lock, the mutex becomes poisoned and
    ///   there is no way to unpoison him, so we can't use one. We thus have to use an RwLock, but
    ///   it does not gives the Sync trait for free, the contained object needs to be Sync itself,
    ///   thus forcing Actors to be Sync.
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

