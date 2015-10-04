use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

trait Actor {
    fn treat_messages(&mut self);
}

struct PrinterActor {
    // VecDeque is the current recommendation for FIFOs.
    callbacks: VecDeque<String>,
    // TODO(gamazeps): Find the proper type for the main queue.
    worker_queue: Arc<RefCell<VecDeque<PrinterActor>>>,
}

impl Actor for PrinterActor {
    fn treat_messages(&mut self) {
        if let Some(s) = self.callbacks.pop_front() {
            println!("{}", s);
        }
    }
}

impl PrinterActor {
    fn new(queue: Arc<RefCell<VecDeque<PrinterActor>>>) -> PrinterActor {
        PrinterActor {
            callbacks: VecDeque::new(),
            worker_queue: queue,
        }
    }

    fn send(&mut self, message: String) {
        self.callbacks.push_back(message);
    }
}

fn main() {
    // Arc allows managing a shared ressource in a thread safe way.
    // RefCell allows sharing a mutable reference.
    let shared_queue: Arc<RefCell<VecDeque<PrinterActor>>> = Arc::new(RefCell::new(VecDeque::new()));
    let mut actor = PrinterActor::new(shared_queue.clone());

    actor.send("Hello world!".to_string());

    actor.treat_messages();
}
