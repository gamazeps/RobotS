use std::collections::VecDeque;

struct PrinterActor {
    callbacks: VecDeque<String>,
}

impl PrinterActor {
    fn new() -> PrinterActor {
        PrinterActor {
            callbacks: VecDeque::new(),
        }
    }

    fn send(&mut self, message: String) {
        self.callbacks.push_back(message);
    }

    fn treat_messages(&mut self) {
        if let Some(s) = self.callbacks.pop_front() {
            println!("{}", s);
        }
    }
}



fn main() {
    let mut actor = PrinterActor::new();

    actor.send("Hello world!".to_string());

    actor.treat_messages();
}
