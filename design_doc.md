# RobotS an actor library for the rust language

## Introduction

This document will present the design of the RobotS library.

This library is highly inspired from the work done in [akka](akka.io) which is itself inspired by
Erlang's processes in the [Eralng OTP](http://www.erlang.org/doc/).

Just like in akka:
  - Actors will have a hierarchy, each actor supervising its children.
  - Actors will be defined by implementing the `Actor` trait and by implementing the `receive`
 method.
  - An ActorSystem will be present.

The value of this project is to combine the advantages of an Actor model and of a type system,
thus catching as much errors at compile time as possible and having guarantees. It would also be
the first implementation of an actor model in rust and is thus also useful in that way.

## Actors creation

We will use the system of `Props` used in akka to have immutable and thread safe factories of
`Actors`, having these is valuable as it allows to recreate / restart actors if they fail.

```rust
struct Props<T: Actor> {
    /// Here will go the fields when options are given for actors creations.
    // This is needed to have genericity over any T if we do not hold any T.
    _phantom: PhantomData<T>
    // TODO(raimundo) a sequence of Any.
}

impl Props<T> {}

actor_ref = actor_system.actor_of(Props::MyActor::new(), "my_actor");
```

An instance of the actor is then created using either actor_system.actor_of(props, name) or
with context.actor_of(props, name), the first one create a top level user actor
and the second one creates a child of the actor's whose context is called.
This gives us a reference to the newly created actor.

## Actors Implementation

In order to implement a new `Actor`, it is necessary to create a `struct` with the attributes of
the actor (if any) and to implement the `Actor` trait for that struct. Note that only the `receive`
method has to be implemented.

```rust
struct MyActor {
    // Fields go here.
}

impl Actor for MyActor {
    fn receive(message: Message, context: &ActorContext) -> () {
        match message.content() {
            case "hello".to_owned() => println!("Oh, hello there"),
            case _ => println!("Haven't your parents told you to greet people ?")
        }
    }
}
```

Note that the context is here passed as an argument, as there is no implicit conversion from an
Actor to an ActorRef in rust (no implicit at all actually).

## Actor References

Just like the actor model imposes, we never manipulate actors, but actor references instead :
  - Sending messages is done by sending to the actor's reference.
  - Creating an actor gives out a reference to this actor.

The actor reference structure looks like the following:

// TODO(gamazeps): add the appropriate concurrency protections.
```rust
struct ActorRef<T: Actor> {
    /// Whether the actor is local or not.
    is_local: bool,
    /// Logical path to the actor.
    path: ActorPath,
    /// 'real' actor.
    actor_cell: ActorCell<T>,
}
```

ActorRef will implement a canReceive and canSend trait, that will contain the code for receiving and
sending messages, this allows us to have futures implement the canReceive trait, and thus receive
messages.

## Actor cell

This is what is used to contain the logic of an actor, i.e its actor system, props, mailbox, sender
information and children and parent ActorRef.

```rust
struct ActorCell<T: Actor> {
    mailbox: Mailbox,
    props: Props<T>,
    actor_system: ActorSystem,
    parent: ActorRef<Stuff>,
    context: ActorContext,
    actor: T
}
```

## Actor System

The actor system is in charge of the whole set of actors, there should only be one per application.

It is the one who creates and starts the three original actors:
  - The root actor.
  - The user actor.
  - The system actor.

The original actor creation have to go by him using actor_system.actor_of(props, name).

## Actor hierarchy

Just like in akka each actor supervises its children.

There are three actors provided by the actor system which are very important.

### Root actor

He is the one and original actor of the system and thus all actors are its children.

He is the one responsible for monitoring the whole system, terminating him terminates the whole
system.

### User actor

All actors created by the user are its children.

Calling actor_system.actor_of(props, name) will actually have the user actor create it.

This actor is the one responsible for creating the thread pool that will handle the user actors.
This is done this way so that each hierarchy can spawn its own threads, and not having a shared pool
for all actors, this way the system actors (in the system actor's hierarchy) can have their own
thread pool to handle sockets (for remoting), dead letters (for logging), path queries (for getting
an ActorRef from a path), etc...

### System actor

Not to mistake with actor system :p

Having an actor to manage the system related actions allows us to use the power of actors while
implementing actors, thus the event stream is an actor, the DeadMessages mailbox is an actor and
the part responsible for sending messages accross machines is also an actor (with its dedicated
thread pool, as network operations tend to be blocking).

## Futures

In the first version (hopefully it will be improved) futures will be taken from a currently existing
library.

They will also implement the canReceive trait in order to have them act a bit like actorRef and thus
allow the ask pattern (more detail in the actor reference and ask parts).

## Sending messages accross actors

The main (and recommanded) way to have actors commmunicate with each other is the use of the tell
function on actor references.

If an anwser is desired and that it cannot be done by having it sent with a send, the ask pattern
can be used.

### tell

Tell is a way to send a message to an actor with its actor ref, the message is received at most
once.

The pattern is the following:

```rust
let actor_1 = actors_system.actor_of(Props::Myactor::new(), "1");
let actor_2 = actors_system.actor_of(Props::Myactor::new(), "2");

actor_1.tell(actor_2, "Hello");
```

We can also have:

```rust
let actor = actors_system.actor_of(Props::Myactor::new(), "1");
tellTo(actor, "Hi");
```

Or the following inside the receive function:

```rust
impl Actor for MyActor {
    fn receive(message: Message, context: &ActorContext) {
        match message {
            _ => context.tell(context.sender(), "Hello friend")
        }
    }
}
```

Where tellTo puts the deadLetters actor (given by the system actor) as the sender.

Note that we have to go through the actor context  compared to Scala, because there are no implicits
in rust, and thus an Actor cannot be implitly casted into an ActorRef.

Having done that, the message will be enqueud in the receiving actor's mailbox, and the actor will
be enqueud in the pool (usually the user actor's pool) of actors with message to handle.

Just like in akka, the context will send the ActorRef of the Actor sending a message.

Just like in akka, the context.sender is updated when handling a message (with the ActorRef put when
sending the message).

Note that having the actor say who sent a message allows message forwarding for free.

### ask

The ask pattern allows to get an answer from an actor in the form of a future.

The ask method returns a future which will contain the answer of the request, and put the future as
the sender of the message.

This will be done by having future implement a CanReceive trait (also implemented by ActorRef) so
that we can do the following on a future:

```rust
struct dummy: Actor {}

impl Actor for dummy {
    fn receive(message: Message, context: &ActorContext) {
        match message {
            _ => context.tell()(context.sender(), "I received your message")
        }
    }
}

let future = askTo(dummy, "Message");
```

The future will be completed when it receives a message.

Note that in this version we will use an already implement version of Futures, later we will want to
have our own to manage their thread consumption (most of the current implementation spawn a thread,
and we do not want to have so many threads).

## Actors addressing

If the actor reference is a reference to a local actor we do nothing special, on the other hand if
it is not a local actor, the message is send passed to a remoting actor in the system actor.

The remoting actor will then forward this message to an actor that has a TCP socket open to the
machine indicated in the ActorPath of the target ActorRef.
If none exists it will create such an Actor.

Note: having a stash (not sure if it will be implemented in the v1 of RobotS) would allow to create
such an actor and send him the message directly afterwards, this way we do not have to wait for the
socket to be open and thus much asynchronicity is gained.

Before sending the message through the socket, the socket actor will update the sender informations
on the message to inform the receiving actor on how to answer (update the path and sets the actor as
non local).

## Remoting

Sending message to remote actor systems is done with a TCP socket as explained above(tcp is chosen
in order to guarantee that messages send by the an actor A to an ActorRef B are received in the
order they are sent).

Receiving messages is done the same way with a TCP socket, the actorRef of the receiver of the
message is created with an identify request (the sender cannnot have a local refernce to this actor
and it thus has to be created).

## Getting an ActorRef from an ActorPath

This will be done exactly as in akka.

## Actor supervision

This will be done exactly as in akka with the same preStart, postStop, preRestart and postRestart
methods.
