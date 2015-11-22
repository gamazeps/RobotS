pub enum Message {
    Dummy,
    Text(String),
}

pub trait Actor: Sync {
    fn receive(&self, message: Message);

    fn pre_start(&self) {}

    fn post_stop(&self) {}

    fn pre_restart(&self) {
        self.post_stop()
    }

    fn post_restart(&self) {
        self.pre_start()
    }
}

