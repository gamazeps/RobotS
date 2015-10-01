# Basic Roadmap

Here are the first steps to get a working prototype of an actor system.
Once these are done, a good skeleton will be build on which it will be possible
to explore new improvements.

## Week 1

Create workers that are able to receive messages in a type safe manner on a single machine.
This implies:
  * Having a macro system for generating the messages in a type safe manner.
  * [optional] Use serde for serializing the messages.
  * Here actors are just threads, the rationale behind that is to have a working messaging
    to be able to test early.

Deliverable: 
  * Be able to create an actor from a binary and send him different messages,
    the messages must be of the correct type and compilation must fail if this is not the case.
  * Examples: create a 'printer' actor, and send him 'hello world'(String type) and 3 (u32) to be printed,
    the first one must work and not the second one.

## Week 2

Implement a way for actors to send messages to each others in a type safe way.
The type safe implemlentation will probably be done using traits for different types of messages.
Start implementing the K to N system (allows lightweight workers).

Deliverable:
  * Have an actor send a message asking for a string to be printed to another actor, and have the
    second actor send an ACK to the first one (easy way to check that transmission works both ways).

## Week 3

Finish the K to N system.

Deliverable:
  * Spawn a lot (TBD) of actors and avoid usig all the RAM, while keeping the previous properties.

## Week 4

Have an hypervisor system to relaunch and recreate the workers.
Find a way to deal with statefull workers (and find if we support them).

Deliverable:
  * Have a worker that fails when it receive a particular message, send it once and then send a String
    to be printed, the string must be printed.

# Future plans

From there multiple paths are open:
  * Work on network capabilities, and spawning actors on other machines (the hypervision will thus become a bit more complex).
  * Improve the hypervision system by making each actor responsible for the actors it created (closer to akka).
