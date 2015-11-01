#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

mod actors;

use std::thread;

use actors::*;
use actors::sample_actors::*;

fn main() {
    let string_data = Message::Data(Box::new("This is a command".to_owned()));
    let uint_data = Message::Data(Box::new(5u32));

    let actor_system = ActorSystem::new();
    let actor_ref_1 = actor_system.spawn_actor::<Printer>("actor_1".to_owned(), Vec::new());
    let actor_ref_2 = actor_system.spawn_actor::<Counter>("actor_2".to_owned(), vec![actor_ref_1.clone()]);
    let actor_ref_3 = actor_system.spawn_actor::<Printer>("actor_3".to_owned(), vec![actor_ref_2.clone()]);

    {
        let actor = actor_ref_2.lock().unwrap();
        actor.send_to_first(string_data);
        actor.send_to_first(uint_data);
        actor.send_to_first(Message::Command);
    }

    {
        let actor = actor_ref_3.lock().unwrap();
        let string_data = Message::Data(Box::new("This is a command".to_owned()));
        let uint_data = Message::Data(Box::new(5u32));
        actor.send_to_first(string_data);
        actor.send_to_first(uint_data);
        actor.send_to_first(Message::Command);
    }

    ActorSystem::spawn_thread(actor_system.clone());

    thread::sleep_ms(3000);

    actor_system.terminate_thread();

    {
        let actor = actor_ref_1.lock().unwrap();
        actor.send_to_first(Message::Command);
    }
    thread::sleep_ms(3000);
}
