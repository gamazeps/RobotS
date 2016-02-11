# RobotS

[![Build Status](https://travis-ci.org/gamazeps/RobotS.svg?branch=travis__test)](https://travis-ci.org/gamazeps/RobotS)
[![Coverage Status](https://coveralls.io/repos/gamazeps/RobotS/badge.svg?branch=master&service=github)](https://coveralls.io/github/gamazeps/RobotS?branch=master)

Robots is a pure rust actor system library, it takes a strong inspiration from [erlang](https://www.erlang.org/) and
[akka](http://akka.io).

Documentation can be found [here](http://gamazeps.github.io/RobotS/).

## Build it

This library can be built on a stable rustc (starting at 1.4.0) version.

A simple
```bash
cargo build
```
should be enough.

## Usage

### Initiate the ActorSystem

In order to use this crate you first need to create an ActorSystem and give it some threads to work
with.

```rust
extern crate robots;
use robots::actors::ActorSystem;

fn main() {
    let actor_system = ActorSystem::new("test".to_owned());
    actor_system.spawn_threads(1);
}
```

You can also shut down the actor system by calling the `shutdown` method on it, it will  stop all
threads and terminate all the actors and their allocated ressources (if you implemented their drop
properly).

That's nice but you need to populate it with some Actors.

### Implement an Actor

In order to define an actor you need to:

  * Define a struct and have it implement the `Actor` trait.
  * Have a function with a single argument to create it (a `tuple` will do that if you need multiple
    values, or `()` if you need none).

```rust
use robots::actors::{Actor, ActorCell, ActorContext};

struct Dummy;

impl Actor for Dummy {
    fn receive(&self, _message: Box<Any>, _context: ActorCell) {}
}

impl Dummy {
    fn new(_: ()) -> Dummy {
        Dummy
    }
}
```

### Instantiate an Actor

Now let's see how to instantiate such an actor.

Actors are created with a `Props`, this is a structure containing a constructor for this actor and
the argument for it.

There are two ways to instantiate an actor, it can either be the child of another user created actor
or be the child of the root actor for user actors.

The first way looks like:

```rust
let props = Props::new(Arc::new(Dummy::new), ());
let greeter = context.actor_of(props, "dummy".to_owned());
```

The second looks like:

```rust
let props = Props::new(Arc::new(Dummy::new), ());
let _actor = actor_system.actor_of(props, "dummy".to_owned());
```

The `actor_of` method will give you an `ActorRef` to the created actor.

Note that the first way to create actors is much faster than the second one (about 10 times), so it
should only be used if you want to create a new actor hierachy.

### Handle messages

The actor will receive its messages in the form of `Box<Any>`, this allows the actor to handle
multiple types of message which can be usefull in many cases (forwarding messages for example).

In order to have a concrete type from a `Box<Any>`, you will need to downcast it like in the
following example.

```rust
impl Actor for Printer {
    fn receive(&self, message: Box<Any>, _context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<String>(message) {
            println!("{}", *message);
        }
    }
}
```

As you can see this is rather easy.

If you think that using Box&lt;Any> is very bad and that someone should do terrible things to me, check
[this post](http://gamazeps.github.io/why-boxany.html) before :)

### ActorContext methods

Now let's see how to use the context argument.

This gives most of the communication methods and features expected of an actor:

```rust
/// Returns an ActorRef to the Actor.
fn actor_ref(&self) -> ActorRef;

/// Spawns a child actor.
fn actor_of(&self, props: Arc<ActorFactory>, name: String) -> ActorRef;

/// Sends a Message to the targeted ActorRef.
fn tell<MessageTo: Message>(&self, to: ActorRef, message: MessageTo);

/// Creates a Future, this Future will send the message to the targetted ActorRef (and thus be
/// the sender of the message).
fn ask<MessageTo: Message>(&self, to: ActorRef, message: MessageTo, future_name: String) -> ActorRef;

/// Completes a Future.
fn complete<MessageTo: Message>(&self, to: ActorRef, complete: MessageTo);

/// Tells a future to forward its result to another Actor.
/// The Future is then dropped.
fn forward_result<T: Message>(&self, future: ActorRef, to: ActorRef);

/// Tells a future to forward its result to another Future that will be completed with this
/// result.
/// The Future is then dropped.
fn forward_result_to_future<T: Message>(&self, future: ActorRef, to: ActorRef);

/// Sends the Future a closure to apply on its value, the value will be updated with the output
/// of the closure.
fn do_computation<T: Message, F: Fn(Box<Any + Send>, ActorCell) -> T + Send + Sync + 'static>
    (&self, future: ActorRef, closure: F);

/// Requests the targeted actor to stop.
fn stop(&self, actor_ref: ActorRef);

/// Asks the father of the actor to terminate it.
fn kill_me(&self);

/// Returns an Arc to the sender of the message being handled.
fn sender(&self) -> ActorRef;

/// Father of the actor.
fn father(&self) -> ActorRef;

/// Children of the actor.
fn children(&self) -> Vec<ActorRef>;

/// Lifecycle monitoring, list of monitored actors.
fn monitoring(&self) -> Vec<ActorRef>;

/// Logical path to the actor, such as `/user/foo/bar/baz`
fn path(&self) -> Arc<ActorPath>;

/// Future containing an Option<ActorRef> with an ActtorRef to the Actor with the given logical
/// path.
///
/// The future will have the path: `$actor/$name_request`
fn identify_actor(&self, logical_path: String, request_name: String) -> ActorRef;
```

## Contributing

All contribution are welcome, if you have a feature request don't hesitate to open an issue !

## Features

  * Actor communication in a local context.
  * Actor supervision with an actor hierarchy (each actor supervises its children).
  * Ask pattern using Futures for asynchronous requests.
  * Name resolving (obtaining an ActorRef from a logical path).

## TODO

  * Network communication in a transparent manner.
  * Investigate the performances to shave some microseconds.
  * Your crazy ideas ?

## Benches

Some crude benchs were written for actor creation and local message passing.

Here are the result on my Intel(R) Core(TM) i5-3317U CPU @ 1.70GHz

```
test create_1000_actors                   ... bench:   3,341,549 ns/iter (+/- 173,594)
test send_1000_messages                   ... bench:   1,217,572 ns/iter (+/- 196,141)
```

Nevertheless, I have not yet managed to get akka working on my computer so I don't have anything to
bench against so do what you want with these benches.
