use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use {Actor, ActorRef, ActorSystem, CanReceive, Message, Props};

/// Main interface for accessing the main Actor information (system, mailbox, sender, props...).
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
    /// Creates a new ActorCell.
    pub fn new(actor: A, props: Props<Args, A>, system: ActorSystem) -> ActorCell<Args, A> {
        ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, system)),
        }
    }

    /// Puts a message with its sender in the Actor's mailbox and schedules the Actor.
    pub fn receive_message(&self, message: Message, sender: Arc<CanReceive + Sync>) {
        self.inner_cell.receive_message(message, sender);
        self.enqueue_actor_ref();
    }

    /// Schedules the Actor on a thread.
    fn enqueue_actor_ref(&self) {
        self.inner_cell.system.enqueue_actor(self.actor_ref());
    }

    /// Makes the Actor handle an envelope in its mailbaox.
    pub fn handle_envelope(&self) {
        self.inner_cell.handle_envelope(self.clone());
    }
}

/// This is the API that Actors are supposed to see of their context while handling a message.
pub trait ActorContext<Args: Copy + Sync + Send + 'static, A: Actor + 'static> {
    /// Returns an ActorRef of the Actor.
    fn actor_ref(&self) -> ActorRef<Args, A>;

    /// Spawns an actor.
    ///
    /// Note that the supervision is not yet implemented so it does the same as creating an actor
    /// through the actor system.
    fn actor_of(&self, props: Props<Args, A>) -> ActorRef<Args, A>;

    /// Sends a message to the targeted CanReceive.
    fn tell<T: CanReceive>(&self, to: T, message: Message);

    /// Returns an Arc to the sender of the message being handled.
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
    _props: Props<Args, A>,
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
            _props: props,
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
