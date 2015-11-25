use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use {Actor, ActorRef, ActorSystem, CanReceive, Props};

/// Main interface for accessing the main Actor information (system, mailbox, sender, props...).
pub struct ActorCell<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    inner_cell: Arc<InnerActorCell<Args, M, A>>,
}

impl<Args: Copy + Send + Sync, M: Copy + Send + Sync + 'static + Any, A: Actor<M>> Clone for ActorCell<Args, M, A> {
    fn clone(&self) -> ActorCell<Args, M, A> {
        ActorCell {
            inner_cell: self.inner_cell.clone()
        }
    }
}


impl<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> ActorCell<Args, M, A> {
    /// Creates a new ActorCell.
    pub fn new(actor: A, props: Props<Args, M, A>, system: ActorSystem) -> ActorCell<Args, M, A> {
        ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, system)),
        }
    }

    /// Puts a message with its sender in the Actor's mailbox and schedules the Actor.
    pub fn receive_message(&self, message: M, sender: Arc<CanReceive >) {
        self.inner_cell.receive_message(message, sender);
        self.inner_cell.system.enqueue_actor(self.actor_ref());
    }

    /// Makes the Actor handle an envelope in its mailbaox.
    pub fn handle_envelope(&self) {
        self.inner_cell.handle_envelope(self.clone());
    }
}

/// This is the API that Actors are supposed to see of their context while handling a message.
pub trait ActorContext<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> {
    /// Returns an ActorRef of the Actor.
    fn actor_ref(&self) -> ActorRef<Args, M, A>;

    /// Spawns an actor.
    ///
    /// Note that the supervision is not yet implemented so it does the same as creating an actor
    /// through the actor system.
    fn actor_of(&self, props: Props<Args, M, A>) -> ActorRef<Args, M, A>;

    /// Sends a Message to the targeted CanReceive<M>.
    fn tell<Message: Copy + Send + Sync + 'static + Any, T: CanReceive>(&self, to: T, message: Message);

    /// Returns an Arc to the sender of the message being handled.
    // NOTE: FUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUU
    fn sender(&self) -> Arc<CanReceive >;
}

impl<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> ActorContext<Args, M, A> for ActorCell<Args, M, A> {
    fn actor_ref(&self) -> ActorRef<Args, M, A> {
        ActorRef::with_cell(self.clone())
    }

    fn actor_of(&self, props: Props<Args, M, A>) -> ActorRef<Args, M, A> {
        let actor = props.create();
        let actor_cell  = ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, self.inner_cell.system.clone())),
        };
        ActorRef::with_cell(actor_cell)
    }

    fn tell<Message: Copy + Send + Sync + 'static + Any, T: CanReceive>(&self, to: T, message: Message) {
        to.receive(Box::new(message), Arc::new(self.actor_ref()));
    }

    fn sender(&self) -> Arc<CanReceive > {
        self.inner_cell.current_sender.lock().unwrap().as_ref().unwrap().clone()
    }
}

struct InnerActorCell<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> {
    actor: Mutex<A>,
    mailbox: Mutex<VecDeque<Envelope<M>>>,
    _props: Props<Args, M, A>,
    system: ActorSystem,
    current_sender: Mutex<Option<Arc<CanReceive >>>,
    busy: Mutex<()>,
}

struct Envelope<M> {
    message: M,
    sender: Arc<CanReceive >,
}

impl<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> InnerActorCell<Args, M, A> {
    fn new(actor: A, props: Props<Args, M, A>, system: ActorSystem) -> InnerActorCell<Args, M, A> {
        InnerActorCell {
            actor: Mutex::new(actor),
            mailbox: Mutex::new(VecDeque::new()),
            _props: props,
            system: system,
            current_sender: Mutex::new(None),
            busy: Mutex::new(()),
        }
    }

    fn receive_envelope(&self, envelope: Envelope<M>) {
        self.mailbox.lock().unwrap().push_back(envelope);
    }

    fn receive_message(&self, message: M, sender: Arc<CanReceive >) {
        self.receive_envelope(Envelope{message: message, sender: sender});
    }

    fn handle_envelope(&self, context: ActorCell<Args, M, A>) {
        let envelope = match self.mailbox.lock().unwrap().pop_front() {
            Some(envelope) => envelope,
            None => {
                println!("no envelope in mailbox");
                return;
            }
        };
        // Now we do not want users to be able to touch current_sender while the actor is busy.
        let _lock = self.busy.lock();
        {
            let mut current_sender = self.current_sender.lock().unwrap();
            *current_sender = Some(envelope.sender.clone());
        };
        {
            let actor = self.actor.lock().unwrap();
            actor.receive(envelope.message, context);
        }
    }

}
