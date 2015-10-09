use std::collections::VecDeque;
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;

struct Actor {
    // TODO(gamazeps): Need to add Arc<Mutex<>> for this to be Send.
    ptr: *mut ActorInner,
}

impl Actor {
    fn new(queue: Arc<Mutex<VecDeque<Actor>>>) -> Actor {
        let ptr = Box::new(ActorInner::new(queue));
        Actor {
            ptr: unsafe {mem::transmute(ptr)
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

    fn receive(&self, message: String) {
        self.transmute_inner_mut().receive(message);
    }

    fn treat(&self) {
        self.transmute_inner_mut().treat();
    }

    fn treat_all(&self) {
        self.transmute_inner_mut().treat_all();
    }
}

struct ActorInner {
    messages: VecDeque<String>,
    actor_queue: Arc<Mutex<VecDeque<Actor>>>,
}

impl ActorInner {
    fn new(queue: Arc<Mutex<VecDeque<Actor>>>) -> ActorInner {
        ActorInner {
            messages: VecDeque::new(),
            actor_queue: queue,
        }
    }

    fn receive(&mut self, message: String) {
        self.messages.push_back(message);
        {
            let mut queue = self.actor_queue.lock().unwrap();
            queue.push_back(self.actor());
        }
    }

    fn treat(&mut self) {
        if let Some(s) = self.messages.pop_front() {
            println!("{}", s);
        }
    }

    fn treat_all(&mut self) {
        while let Some(s) = self.messages.pop_front() {
            println!("{}", s);
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
            let mut actor = None;
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
    let actor = Actor::new(actor_queue);

    actor.receive("message 1".to_string());
    actor.receive("message 2".to_string());
    actor.receive("message 3".to_string());

    let handle = spawn_consumer(actor_queue.clone());

    handle.join();

    println!("Hello World");
}
