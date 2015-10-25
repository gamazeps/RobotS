#![feature(plugin)]
#![plugin(clippy)]

mod actors;

use std::sync::Arc;

use std::thread;
use actors::*;

fn main() {
    let data = Message::Data(Box::new("This is a command".to_owned()));
    let bad_data = Message::Data(Box::new(1i32));

    let actor_system = ActorSystem::new();
    let actor_ref_2 = actor_system.spawn_actor("actor_2".to_owned(), Vec::new());
    let actor_ref_3 = actor_system.spawn_actor("actor_3".to_owned(), Vec::new());
    let actor_ref_1 = actor_system.spawn_actor(
        "actor_1".to_owned(), vec![actor_ref_2.clone(), actor_ref_3.clone()]);

    {
        let actor = actor_ref_1.lock().unwrap();
        actor.send_to_first(data);
        actor.send_to_first(bad_data);
        //actor.broadcast(message.clone());
        //actor.broadcast(command.clone());
        //actor.broadcast(bad_data.clone());
    }

    actor_system.spawn_consumer_thread();

    let handle = ActorSystem::spawn_thread(actor_system.clone());
    handle.join();
}
