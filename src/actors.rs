use {ActorCell};

pub enum Message {
    Dummy,
    Text(String),
    Numbers(u32, u32),
}

pub trait Actor: Send + Sync + Sized{
    fn receive<Args: Copy + Sync + Send + 'static>(&self, message: Message, context: ActorCell<Args, Self>);

    fn pre_start(&self) {}

    fn post_stop(&self) {}

    fn pre_restart(&self) {
        self.post_stop()
    }

    fn post_restart(&self) {
        self.pre_start()
    }
}

