use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use {Actor, ActorRef, ActorSystem, CanReceive, Message, Props};

pub struct ActorCell<Args: Copy + Sync + Send + 'static, A: Actor + 'static> {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    inner_cell: Arc<InnerActorCell<Args, A>>,
}

impl<Args:  Copy + Sync + Send, A: Actor> Clone for ActorCell<Args, A> {
    fn clone(&self) -> ActorCell<Args, A> {
        ActorCell {
            inner_cell: self.inner_cell.clone()
        }
    }
}

impl<Args: Copy + Sync + Send + 'static, A: Actor + 'static> ActorCell<Args, A> {
    pub fn new(actor: A, props: Props<Args, A>, system: ActorSystem) -> ActorCell<Args, A> {
        ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, system)),
        }
    }

    pub fn receive_message(&self, message: Message, sender: Arc<CanReceive + Sync>) {
        self.inner_cell.receive_message(message, sender);
        self.enqueue_actor_ref();
    }

    fn enqueue_actor_ref(&self) {
        self.inner_cell.system.enqueue_actor(self.actor_ref());
    }

    pub fn handle_envelope(&self) {
        self.inner_cell.handle_envelope(self.clone());
    }
}

/// This is the API that actors are supposed to see.
pub trait ActorContext<Args: Copy + Sync + Send + 'static, A: Actor + 'static> {
    fn actor_ref(&self) -> ActorRef<Args, A>;
    fn actor_of(&self, props: Props<Args, A>) -> ActorRef<Args, A>;
    fn tell<T: CanReceive>(&self, to: T, message: Message);
    fn sender(&self) -> Arc<CanReceive + Sync>;
}

impl<Args: Copy + Sync + Send + 'static, A: Actor + 'static> ActorContext<Args, A> for ActorCell<Args, A> {
    fn actor_ref(&self) -> ActorRef<Args, A> {
        ActorRef::with_cell(self.clone())
    }

    fn actor_of(&self, props: Props<Args, A>) -> ActorRef<Args, A> {
        let actor = props.create();
        let actor_cell  = ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, self.inner_cell.system.clone())),
        };
        ActorRef::with_cell(actor_cell)
    }

    fn tell<T: CanReceive>(&self, to: T, message: Message) {
        to.receive(message, Arc::new(self.actor_ref()));
    }

    fn sender(&self) -> Arc<CanReceive + Sync> {
        self.inner_cell.current_sender.read().unwrap().as_ref().unwrap().clone()
    }
}

struct InnerActorCell<Args: Copy + Sync + Send + 'static, A: Actor + 'static> {
    actor: A,
    mailbox: Mutex<VecDeque<Envelope>>,
    props: Props<Args, A>,
    system: ActorSystem,
    current_sender: RwLock<Option<Arc<CanReceive + Sync>>>,
    busy: Mutex<()>,
}

struct Envelope {
    message: Message,
    sender: Arc<CanReceive + Sync>,
}

impl<Args: Copy + Sync + Send + 'static, A: Actor + 'static> InnerActorCell<Args, A> {
    fn new(actor: A, props: Props<Args, A>, system: ActorSystem) -> InnerActorCell<Args, A> {
        InnerActorCell {
            actor: actor,
            mailbox: Mutex::new(VecDeque::new()),
            props: props,
            system: system,
            current_sender: RwLock::new(None),
            busy: Mutex::new(()),
        }
    }

    fn receive_envelope(&self, envelope: Envelope) {
        self.mailbox.lock().unwrap().push_back(envelope);
    }

    fn receive_message(&self, message: Message, sender: Arc<CanReceive + Sync>) {
        self.receive_envelope(Envelope{message: message, sender: sender});
    }

    fn handle_envelope(&self, context: ActorCell<Args, A>) {
        // Allows to have a single thread working on an actor at a time, only issue is that threads
        // still enter this function and block.
        // TODO(gamazeps): fix this, obviously.
        let _lock = self.busy.lock();
        let envelope = match self.mailbox.lock().unwrap().pop_front() {
            Some(envelope) => envelope,
            None => {
                println!("no envelope in mailbox");
                return;
            }
        };
        {
            let mut current_sender = self.current_sender.write().unwrap();
            *current_sender = Some(envelope.sender.clone());
        };
        self.actor.receive(envelope.message, context);
    }

}
