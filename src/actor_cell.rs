use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use {Actor, ActorRef, ActorSystem, CanReceive, Props};

/// Special messages issued by the actor system.
#[derive(Clone)]
pub enum SystemMessage {
    /// Restarts the actor by repa=lacing it with a new version created with its Props.
    Restart,

    /// Makes the actor launch its initialisation.
    Start,

    /// Tells an actor that its child failed.
    Failure(Arc<CanReceive>),
}

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
    pub fn new(actor: A, props: Props<Args, M, A>, system: ActorSystem, father: Arc<CanReceive>) -> ActorCell<Args, M, A> {
        ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, system, father)),
        }
    }

    /// Puts a message with its sender in the Actor's mailbox and schedules the Actor.
    pub fn receive_message(&self, message: M, sender: Arc<CanReceive >) {
        self.inner_cell.receive_message(message, sender);
        self.inner_cell.system.enqueue_actor(self.actor_ref());
    }

    /// Puts a system message with its sender in the Actor's system mailbox and schedules the Actor.
    pub fn receive_system_message(&self, system_message: SystemMessage) {
        self.inner_cell.receive_system_message(system_message);
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
    fn actor_of<ArgBis: Copy + Send + Sync + 'static, MBis: Copy + Send + Sync + 'static + Any, ABis: Actor<MBis> + 'static>(&self, props: Props<ArgBis, MBis, ABis>) -> ActorRef<ArgBis, MBis, ABis>;

    /// Sends a Message to the targeted CanReceive<M>.
    fn tell<Message: Copy + Send + Sync + 'static + Any, T: CanReceive>(&self, to: T, message: Message);

    /// Returns an Arc to the sender of the message being handled.
    fn sender(&self) -> Arc<CanReceive >;
}

impl<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> ActorContext<Args, M, A> for ActorCell<Args, M, A> {
    fn actor_ref(&self) -> ActorRef<Args, M, A> {
        ActorRef::with_cell(self.clone())
    }

    fn actor_of<ArgBis: Copy + Send + Sync + 'static, MBis: Copy + Send + Sync + 'static + Any, ABis: Actor<MBis> + 'static>(&self, props: Props<ArgBis, MBis, ABis>) -> ActorRef<ArgBis, MBis, ABis> {
        let actor = props.create();
        let actor_cell  = ActorCell {
            inner_cell: Arc::new(InnerActorCell::new(actor, props, self.inner_cell.system.clone(),
                                                     Arc::new(self.actor_ref()))),
        };
        let child = ActorRef::with_cell(actor_cell);
        child.receive_system_message(SystemMessage::Start);
        child
    }

    fn tell<Message: Copy + Send + Sync + 'static + Any, T: CanReceive>(&self, to: T, message: Message) {
        to.receive(Box::new(message), Arc::new(self.actor_ref()));
    }

    fn sender(&self) -> Arc<CanReceive > {
        self.inner_cell.current_sender.lock().unwrap().as_ref().unwrap().clone()
    }
}

struct Failsafe {
    father: Arc<CanReceive>,
    child: Arc<CanReceive>,
    active: bool,
}

impl Failsafe {
    fn new(father: Arc<CanReceive>, child: Arc<CanReceive>) -> Failsafe {
        Failsafe {
            father: father,
            child: child,
            active: true,
        }
    }

    fn cancel(mut self) {
        self.active = false;
    }
}

impl Drop for Failsafe {
    fn drop(&mut self) {
        if self.active {
            self.father.receive_system_message(SystemMessage::Failure(self.child.clone()));
        }
    }
}

struct InnerActorCell<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> {
    actor: RwLock<A>,
    mailbox: Mutex<VecDeque<Envelope<M>>>,
    system_mailbox: Mutex<VecDeque<SystemMessage>>,
    props: Props<Args, M, A>,
    system: ActorSystem,
    current_sender: Mutex<Option<Arc<CanReceive >>>,
    busy: Mutex<()>,
    father: Arc<CanReceive>,
}

struct Envelope<M> {
    message: M,
    sender: Arc<CanReceive>,
}

impl<Args: Copy + Send + Sync + 'static, M: Copy + Send + Sync + 'static + Any, A: Actor<M> + 'static> InnerActorCell<Args, M, A> {
    fn new(actor: A, props: Props<Args, M, A>, system: ActorSystem, father: Arc<CanReceive>) -> InnerActorCell<Args, M, A> {
        InnerActorCell {
            actor: RwLock::new(actor),
            mailbox: Mutex::new(VecDeque::new()),
            system_mailbox: Mutex::new(VecDeque::new()),
            props: props,
            system: system,
            current_sender: Mutex::new(None),
            busy: Mutex::new(()),
            father: father,
        }
    }

    fn receive_envelope(&self, envelope: Envelope<M>) {
        self.mailbox.lock().unwrap().push_back(envelope);
    }

    fn receive_message(&self, message: M, sender: Arc<CanReceive>) {
        self.receive_envelope(Envelope{message: message, sender: sender});
    }

    fn receive_system_message(&self, system_message: SystemMessage) {
        self.system_mailbox.lock().unwrap().push_back(system_message);
    }

    fn handle_envelope(&self, context: ActorCell<Args, M, A>) {
        // Now we do not want users to be able to touch current_sender while the actor is busy.
        let _lock = self.busy.lock();
        let failsafe = Failsafe::new(self.father.clone(), Arc::new(context.actor_ref()));
        // System messages are handled first, so that we can restart an actor if he failed without
        // loosing the messages in the mailbox.
        // NOTE: This does not break the fact that messages sent by the same actor are treated in
        // the order they are sent (if all to the same target actor), as system messages must not
        // be sent by other actors by the user.
        if let Some(message) = self.system_mailbox.lock().unwrap().pop_front() {
            match message {
                SystemMessage::Restart => self.restart(),
                SystemMessage::Start => self.start(),
                SystemMessage::Failure(actor) => actor.receive_system_message(SystemMessage::Restart),
            }
            failsafe.cancel();
            return;
        }
        let envelope = match self.mailbox.lock().unwrap().pop_front() {
            Some(envelope) => envelope,
            None => {
                failsafe.cancel();
                return;
            }
        };
        {
            let mut current_sender = self.current_sender.lock().unwrap();
            *current_sender = Some(envelope.sender.clone());
        };
        {
            let actor = self.actor.read().unwrap();
            actor.receive(envelope.message, context);
        }
        failsafe.cancel();
    }

    fn start(&self) {
        self.actor.write().unwrap().pre_start();
    }

    fn restart(&self) {
        let mut actor = self.actor.write().unwrap();
        actor.pre_restart();
        *actor = self.props.create();
        actor.post_restart();
    }
}
