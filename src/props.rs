use std::marker::PhantomData;
use std::sync::Arc;

use {Actor};

pub struct Props<Args: Copy + Sync, A: Actor> {
    _phantom: PhantomData<A>,
    creator: Arc<Fn(Args) -> A + Sync + Send>,
    args: Args,
}

impl<Args: Copy + Sync, A: Actor> Props<Args, A> {
    pub fn new(creator: Arc<Fn(Args) -> A + Sync + Send>, args: Args) -> Props<Args, A> {
        Props::<Args, A> {
            _phantom: PhantomData,
            creator: creator,
            args: args,
        }
    }

    pub fn create(&self) -> A {
        // TODO(gamazeps): reopen https://github.com/rust-lang/rust/issues/18343 with an example.
        let args = self.args;
        (self.creator)(args)
    }
}
