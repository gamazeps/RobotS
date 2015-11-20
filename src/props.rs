use std::marker::PhantomData;

use {Actor};

pub struct Props<Args: Copy, A: Actor> {
    _phantom: PhantomData<A>,
    creator: &'static Fn(Args) -> A,
    args: Args,
}

impl<Args: Copy, A: Actor> Props<Args, A> {
    fn new(creator: &'static Fn(Args) -> A, args: Args) -> Props<Args, A> {
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
