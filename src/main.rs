use std::collections::VecDeque;
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;

enum Message {
    Data(String),
    Stop,
}

struct Actor {
    // TODO(gamazeps): Need to add Arc<Mutex<>> for this to be Send.
    // Use Unique instead (or maybe not...)
    ptr: *mut ActorInner,
}

impl Actor {
    fn new(queue: Arc<Mutex<VecDeque<Actor>>>) -> Actor {
        // Memory is allocated for an ActorInner.
        let ptr = Box::new(ActorInner::new(queue));
        // Ownership is passed to the created object, it will be deallocated when the object is
        // dropped.
        Actor {
            ptr: unsafe {
                mem::transmute(ptr)
            }
        }
    }

    fn inner(&self) -> &ActorInner {
        unsafe {&*self.ptr}
    }

    fn inner_mut(&mut self) -> &mut ActorInner {
        unsafe {&mut *self.ptr}
    }

    fn transmute_inner_mut(&self) -> &mut ActorInner {
        unsafe {&mut *self.ptr}
    }

    fn receive(&self, message: Message) {
        self.transmute_inner_mut().receive(message);
    }

    fn treat(&self) {
        self.transmute_inner_mut().treat();
    }

    fn treat_all(&self) {
        self.transmute_inner_mut().treat_all();
    }
}

unsafe impl Send for Actor {}

// Note that this struct should not be instantied by hand.
// TODO(gamazeps): create a module to have enforce this.
// An InnerActor  is dropped when the Actor that created it (with new) is dropped.
// This means that if the Actor goes out of scope while it still has a message to treat,
// there will be a segfault (or at least UB).
// This is a design choice for now (otherwise we just need to take inspiration from the code in Arc).
struct ActorInner {
    messages: VecDeque<Message>,
    actor_queue: Arc<Mutex<VecDeque<Actor>>>,
}

impl ActorInner {
    fn new(queue: Arc<Mutex<VecDeque<Actor>>>) -> ActorInner {
        ActorInner {
            messages: VecDeque::new(),
            actor_queue: queue,
        }
    }

    fn receive(&mut self, message: Message) {
        self.messages.push_back(message);
        {
            let mut queue = self.actor_queue.lock().unwrap();
            queue.push_back(self.actor());
        }
    }

    // TODO(gamazeps): There might be a useless copy of data, deal with it later.
    fn treat_message(&self, message: Message) {
        match message {
            Message::Data(s) => println!("{}", s),
            Message::Stop => println!("Stop received"),
        }
    }


    fn treat(&mut self) {
        if let Some(message) = self.messages.pop_front() {
            self.treat_message(message);
        }
    }

    fn treat_all(&mut self) {
        while let Some(message) = self.messages.pop_front() {
            self.treat_message(message);
        }
    }

    fn actor(&self) -> Actor {
        Actor{
            ptr: unsafe {
                mem::transmute(self)
            }
        }
    }
}

fn spawn_consumer(actor_queue: Arc<Mutex<VecDeque<Actor>>>) -> thread::JoinHandle<usize> {
    thread::spawn(move || {
        loop {
            let actor;
            {
                let mut queue = actor_queue.lock().unwrap();
                actor = queue.pop_front();
            }
            if let Some(actor) = actor {
                actor.treat();
            }
        }
    })
}


fn main() {
    let actor_queue = Arc::new(Mutex::new(VecDeque::new()));
    let actor_1 = Actor::new(actor_queue.clone());
    let actor_2 = Actor::new(actor_queue.clone());

    actor_1.receive(Message::Data("message 1".to_string()));
    actor_2.receive(Message::Data("message 2".to_string()));
    actor_1.receive(Message::Data("message 3".to_string()));
    actor_2.receive(Message::Data("message 4".to_string()));

    let handle = spawn_consumer(actor_queue.clone());

    handle.join();

    println!("Hello World");
}
