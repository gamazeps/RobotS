use std::marker::PhantomData;

use {Actor};

pub struct Props<Args, A: Actor> {
    _phantom: PhantomData<A>,
    creator: &'static Fn(Args) -> A,
    args: Args,
}

impl<Args, A: Actor> Props<Args, A> {
    fn new(creator: &'static Fn(Args) -> A, args: Args) -> Props<Args, A> {
        Props::<Args, A> {
            _phantom: PhantomData,
            creator: creator,
            args: args,
        }
    }

    fn create(&self) -> A {
        self.creator(self.args)
    }
}
