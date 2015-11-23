use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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

    pub fn receive_message(&self, message: Message) {
        self.inner_cell.receive_message(message);
        self.enqueue_actor_ref();
    }

    fn enqueue_actor_ref(&self) {
        self.inner_cell.system.enqueue_actor(self.actor_ref());
    }

    pub fn handle_envelope(&self) {
        self.inner_cell.handle_envelope();
    }
}

/// This is the API that actors are supposed to see.
trait ActorContext<Args: Copy + Sync + Send + 'static, A: Actor + 'static> {
    fn actor_ref(&self) -> ActorRef<Args, A>;
    fn actor_of(&self, props: Props<Args, A>) -> ActorRef<Args, A>;
    fn tell<T: CanReceive>(&self, to: T, message: Message);
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
        to.receive(message);
    }
}

struct InnerActorCell<Args: Copy + Sync + Send + 'static, A: Actor + 'static> {
    actor: A,
    mailbox: Mutex<VecDeque<Envelope>>,
    props: Props<Args, A>,
    system: ActorSystem,
}

struct Envelope {
    message: Message,
}

impl<Args: Copy + Sync + Send + 'static, A: Actor + 'static> InnerActorCell<Args, A> {
    fn new(actor: A, props: Props<Args, A>, system: ActorSystem) -> InnerActorCell<Args, A> {
        InnerActorCell {
            actor: actor,
            mailbox: Mutex::new(VecDeque::new()),
            props: props,
            system: system,
        }
    }

    fn receive_envelope(&self, envelope: Envelope) {
        self.mailbox.lock().unwrap().push_back(envelope);
    }

    fn receive_message(&self, message: Message) {
        self.receive_envelope(Envelope{message: message});
    }

    fn handle_envelope(&self) {
        let envelope = match self.mailbox.lock().unwrap().pop_front() {
            Some(envelope) => envelope,
            None => {
                println!("no envelope in mailbox");
                return;
            }
        };
        self.actor.receive(envelope.message);
    }

}
