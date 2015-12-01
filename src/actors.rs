use std::any::Any;

use ActorCell;

/// Trait to be implemented by messages and args, this is automatically given if a struct is
/// already `Copy + Send + Sync + 'static + Any`.
pub trait Message: Copy + Send + Sync + 'static + Any {}
impl<T> Message for T where T: Copy + Send + Sync + 'static + Any {}

/// This is the trait to implement to become an Actor.
///
/// Normaly only the receive method has to be implemented.
pub trait Actor<M: Message>: Send + Sync + Sized {

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
    ///     fn receive<Args: Message>
    ///         (&self, message: Message, _context: ActorCell<Args, MyActor>) {
    ///         match message {
    ///             Message::Dummy => context.tell(context.sender(), Message::Dummy),
    ///             _ => println!("Hi !"),
    ///         }
    ///     }
    /// }
    fn receive<Args: Message>(&self, message: M, context: ActorCell<Args, M, Self>);

    /// Method called before the Actor is started.
    fn pre_start(&self) {
    }

    /// Method called after the Actor is stopped.
    fn post_stop(&self) {
    }

    /// Method called before the Actor is restarted.
    fn pre_restart(&self) {
        self.post_stop();
    }

    /// Method called after the Actor is restarted.
    fn post_restart(&self) {
        self.pre_start();
    }
}

