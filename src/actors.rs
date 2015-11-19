use std::any::Any;

pub enum Message {
    Dummy,
}

pub trait Actor: Sync {
    fn receive(&self, message: Message);

    fn preStart(&self) {}

    fn postStop(&self) {}

    fn preRestart(&self) {
        self.postStop()
    }

    fn postRestart(&self) {
        self.preStart()
    }
}

