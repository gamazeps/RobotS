use std::marker::PhantomData;
use std::sync::Arc;

use Actor;

/// Factory for `A`.
///
/// It will always create an `A` with the same function and arguments.
///
/// It is also thread safe, and thus we can respawn an Actor across different threads.
pub struct Props<Args: Copy + Sync, A: Actor> {
    _phantom: PhantomData<A>,
    creator: Arc<Fn(Args) -> A + Sync + Send>,
    args: Args,
}

impl<Args: Copy + Sync, A: Actor> Props<Args, A> {
    /// Creates a `Props` which is a factory for `A` with the `creator` function and `args` args.
    pub fn new(creator: Arc<Fn(Args) -> A + Sync + Send>, args: Args) -> Props<Args, A> {
        Props::<Args, A> {
            _phantom: PhantomData,
            creator: creator,
            args: args,
        }
    }

    /// Creates an Actor instance with the `creator` function and the `args` args.
    ///
    /// This is meant to allow to respawn an Actor when it fails.
    pub fn create(&self) -> A {
        // TODO(gamazeps): reopen https://github.com/rust-lang/rust/issues/18343 with an example.
        let args = self.args;
        (self.creator)(args)
    }
}
