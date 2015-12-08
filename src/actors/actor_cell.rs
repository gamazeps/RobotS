use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock, Weak};

use actors::{Actor, ActorRef, ActorSystem, CanReceive, Message, Props};

enum Ref<T> {
    StrongRef(Arc<T>),
    WeakRef(Weak<T>),
}

macro_rules! unwrap_inner {
    ($r:expr, $b:block) => {
        match $r {
            Ref::StrongRef(ref inner) => inner.clone(),
            Ref::WeakRef(ref inner) => match inner.upgrade() {
                Some(inner) => inner.clone(),
                None => {
                    $b
                },
            }
        }
    }
}

/// Main interface for accessing the main Actor information (system, mailbox, sender, props...).
pub struct ActorCell<Args: Message, M: Message, A: Actor<M> + 'static> {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    inner_cell: Ref<InnerActorCell<Args, M, A>>,
}

impl<Args: Message, M: Message, A: Actor<M>> Clone for ActorCell<Args, M, A> {
    fn clone(&self) -> ActorCell<Args, M, A> {
        ActorCell {
            inner_cell: Ref::WeakRef(match self.inner_cell {
                Ref::StrongRef(ref inner) => Arc::downgrade(&inner),
                Ref::WeakRef(ref inner) => inner.clone(),
            })
        }
    }
}


impl<Args: Message, M: Message, A: Actor<M> + 'static> ActorCell<Args, M, A> {
    /// Creates a new ActorCell.
    pub fn new(actor: A, props: Props<Args, M, A>, system: ActorSystem, father: Arc<CanReceive>, name: Arc<String>) -> ActorCell<Args, M, A> {
        ActorCell {
            inner_cell: Ref::StrongRef(Arc::new(InnerActorCell::new(actor, props, system, father, name))),
        }
    }

    /// Puts a message with its sender in the Actor's mailbox and schedules the Actor.
    pub fn receive_message(&self, message: InnerMessage<M>, sender: Arc<CanReceive >) {
        let inner = unwrap_inner!(self.inner_cell,
                                {
                                    println!("A message was send to a ref to a stopped actor");
                                    return;
                                });
        inner.receive_message(message, sender);
        inner.system.enqueue_actor(self.actor_ref());
    }

    /// Puts a system message with its sender in the Actor's system mailbox and schedules the Actor.
    pub fn receive_system_message(&self, system_message: SystemMessage) {
        let inner = unwrap_inner!(self.inner_cell,
                                {
                                    println!("A message was send to a ref to a stopped actor");
                                    return;
                                });
        inner.receive_system_message(system_message);
        inner.system.enqueue_actor(self.actor_ref());
    }

    /// Makes the Actor handle an envelope in its mailbox.
    pub fn handle_envelope(&self) {
        let inner = unwrap_inner!(self.inner_cell,
                                {
                                    println!("A message was send to a ref to a stopped actor");
                                    return;
                                });
        inner.handle_envelope(self.clone());
    }
}

/// This is the API that Actors are supposed to see of their context while handling a message.
pub trait ActorContext<Args: Message, M: Message, A: Actor<M> + 'static> {
    /// Returns an ActorRef of the Actor.
    fn actor_ref(&self) -> Arc<ActorRef<Args, M, A>>;

    /// Spawns an actor.
    ///
    /// Note that the supervision is not yet implemented so it does the same as creating an actor
    /// through the actor system.
    fn actor_of<ArgsBis: Message, MBis: Message, ABis: Actor<MBis> + 'static>(&self, props: Props<ArgsBis, MBis, ABis>, name: String) -> Arc<ActorRef<ArgsBis, MBis, ABis>>;

    /// Sends a Message to the targeted CanReceive<M>.
    fn tell<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo);

    /// Requests the targeted actor to stop.
    fn stop(&self, actor_ref: Arc<CanReceive>);

    /// Asks the father to kill the actor.
    fn kill_me(&self);

    /// Returns an Arc to the sender of the message being handled.
    fn sender(&self) -> Arc<CanReceive>;

    /// Father of the actor.
    fn father(&self) -> Arc<CanReceive>;

    /// Children of the actor.
    fn children(&self) -> Vec<Arc<CanReceive>>;

    /// Lifecycle monitoring, list of monitored actors.
    fn monitoring(&self) -> Vec<Arc<CanReceive>>;

    /// Lifecycle monitoring, list of monitored actors.
    fn path(&self) -> Arc<String>;
}

impl<Args: Message, M: Message, A: Actor<M> + 'static> ActorContext<Args, M, A> for ActorCell<Args, M, A> {
    fn actor_ref(&self) -> Arc<ActorRef<Args, M, A>> {
        Arc::new(ActorRef::with_cell(self.clone(), self.path()))
    }

    fn actor_of<ArgsBis: Message, MBis: Message, ABis: Actor<MBis> + 'static>(&self, props: Props<ArgsBis, MBis, ABis>, name: String) -> Arc<ActorRef<ArgsBis, MBis, ABis>> {
        let inner = unwrap_inner!(self.inner_cell,
                                  {
                                    panic!("Tried to create an actor from the context of a no longer
                                           existing actor");
                                  });
        let actor = props.create();
        let name = Arc::new(name);
        let inner_cell = InnerActorCell::new(actor, props, inner.system.clone(), self.actor_ref(), name.clone());
        let actor_cell = ActorCell {
            inner_cell: Ref::StrongRef(Arc::new(inner_cell)),
        };
        let internal_ref = ActorRef::with_cell(actor_cell, name.clone());
        let external_ref = Arc::new(internal_ref.clone());
        {inner.children.lock().unwrap().push((name.clone(), Arc::new(internal_ref)));}
        {inner.monitoring.lock().unwrap().push(external_ref.clone());}
        external_ref.receive_system_message(SystemMessage::Start);
        external_ref
    }

    fn tell<MessageTo: Message>(&self, to: Arc<CanReceive>, message: MessageTo) {
        to.receive(Box::new(message), self.actor_ref());
    }

    fn sender(&self) -> Arc<CanReceive > {
        let inner = unwrap_inner!(self.inner_cell,
                                  {
                                    panic!("Tried to get a sender from the context of a no longer
                                           existing actor");
                                  });
        let current_sender = inner.current_sender.lock().unwrap().as_ref().unwrap().clone();
        current_sender
    }

    fn stop(&self, actor_ref: Arc<CanReceive>) {
        actor_ref.receive(Box::new(ControlMessage::PoisonPill), self.actor_ref());
    }

    fn kill_me(&self) {
        self.father().receive(Box::new(ControlMessage::KillMe(self.actor_ref())), self.actor_ref());
    }

    fn father(&self) -> Arc<CanReceive> {
        let inner = unwrap_inner!(self.inner_cell,
                                  {
                                      panic!("Tried to get the father from the context of a no
                                             longer existing actor");
                                  });
        inner.father.clone()
    }

    fn children(&self) -> Vec<Arc<CanReceive>> {
        let inner = unwrap_inner!(self.inner_cell,
                                  {
                                      panic!("Tried to get the children from the context of a no
                                             longer existing actor");
                                  });
        let mut res = Vec::new();
        for child in inner.children.lock().unwrap().iter() {
            res.push(child.1.clone());
        }
        res
    }

    fn monitoring(&self) -> Vec<Arc<CanReceive>> {
        let inner = unwrap_inner!(self.inner_cell,
                                  {
                                      panic!("Tried to get the monitored actors from the context of
                                             a no longer existing actor");
                                  });
        let monitoring = inner.monitoring.lock().unwrap().clone();
        monitoring
    }

    fn path(&self) -> Arc<String> {
        let inner = unwrap_inner!(self.inner_cell,
                                  {
                                      panic!("Tried to get the monitored actors from the context of
                                             a no longer existing actor");
                                  });
        inner.name.clone()
    }
}

#[derive(PartialEq)]
enum ActorState {
    Failed,
    Running,
    Unstarted,
}

struct Failsafe {
    father: Arc<CanReceive>,
    child: Arc<CanReceive>,
    state: Arc<RwLock<ActorState>>,
    active: bool,
}

impl Failsafe {
    fn new(father: Arc<CanReceive>, child: Arc<CanReceive>, state: Arc<RwLock<ActorState>>) -> Failsafe {
        Failsafe {
            father: father,
            child: child,
            state: state,
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
            *self.state.write().unwrap() = ActorState::Failed;
            self.father.receive_system_message(SystemMessage::Failure(self.child.clone()));
        }
    }
}

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

struct Envelope<M: Message> {
    message: InnerMessage<M>,
    sender: Arc<CanReceive>,
}

/// Types of message that can be sent to an actor that will be treated normally.
#[derive(Clone)]
pub enum InnerMessage<M: Message> {
    /// Regular message.
    Message(M),

    /// Control messages.
    Control(ControlMessage),
}

/// Control Messages.
#[derive(Clone)]
pub enum ControlMessage {
    /// Requests the termination of an actor.
    PoisonPill,

    /// Message sent to monitoring actors when an actpr is terminated.
    Terminated(Arc<CanReceive>),

    /// Message sent to the father of an actor to request being dropped.
    KillMe(Arc<CanReceive>),
}

struct InnerActorCell<Args: Message, M: Message, A: Actor<M> + 'static> {
    actor: RwLock<A>,
    mailbox: Mutex<VecDeque<Envelope<M>>>,
    system_mailbox: Mutex<VecDeque<SystemMessage>>,
    props: Props<Args, M, A>,
    system: ActorSystem,
    name: Arc<String>,
    current_sender: Mutex<Option<Arc<CanReceive>>>,
    busy: Mutex<()>,
    father: Arc<CanReceive>,
    children: Mutex<Vec<(Arc<String>, Arc<CanReceive>)>>,
    monitoring: Mutex<Vec<Arc<CanReceive>>>,
    actor_state: Arc<RwLock<ActorState>>,
    _monitored: Mutex<Vec<Arc<CanReceive>>>,
}

impl<Args: Message, M: Message, A: Actor<M> + 'static> InnerActorCell<Args, M, A> {
    fn new(actor: A, props: Props<Args, M, A>, system: ActorSystem, father: Arc<CanReceive>, name: Arc<String>) -> InnerActorCell<Args, M, A> {
        InnerActorCell {
            actor: RwLock::new(actor),
            mailbox: Mutex::new(VecDeque::new()),
            system_mailbox: Mutex::new(VecDeque::new()),
            props: props,
            system: system,
            name: name,
            current_sender: Mutex::new(None),
            busy: Mutex::new(()),
            father: father.clone(),
            children: Mutex::new(Vec::new()),
            monitoring: Mutex::new(Vec::new()),
            actor_state: Arc::new(RwLock::new(ActorState::Unstarted)),
            _monitored: Mutex::new(vec![father.clone()]),
        }
    }

    fn receive_envelope(&self, envelope: Envelope<M>) {
        self.mailbox.lock().unwrap().push_back(envelope);
    }

    fn receive_message(&self, message: InnerMessage<M>, sender: Arc<CanReceive>) {
        self.receive_envelope(Envelope{message: message, sender: sender});
    }

    fn receive_system_message(&self, system_message: SystemMessage) {
        self.system_mailbox.lock().unwrap().push_back(system_message);
    }

    fn handle_envelope(&self, context: ActorCell<Args, M, A>) {
        // Now we do not want users to be able to touch current_sender while the actor is busy.
        let _lock = self.busy.lock();
        let failsafe = Failsafe::new(self.father.clone(), context.actor_ref(), self.actor_state.clone());
        // System messages are handled first, so that we can restart an actor if he failed without
        // loosing the messages in the mailbox.
        // NOTE: This does not break the fact that messages sent by the same actor are treated in
        // the order they are sent (if all to the same target actor), as system messages must not
        // be sent by other actors by the user.
        if let Some(message) = self.system_mailbox.lock().unwrap().pop_front() {
            match message {
                SystemMessage::Restart => self.restart(context),
                SystemMessage::Start => self.start(context),
                SystemMessage::Failure(actor) => actor.receive_system_message(SystemMessage::Restart),
            }
            failsafe.cancel();
            return;
        }

        if *self.actor_state.read().unwrap() == ActorState::Running {
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
                match envelope.message {
                    InnerMessage::Message(message) => actor.receive(message, context),
                    InnerMessage::Control(message) => match message {
                        ControlMessage::PoisonPill => context.kill_me(),
                        ControlMessage::Terminated(_) => actor.receive_termination(context),
                        ControlMessage::KillMe(actor_ref) => self.kill(actor_ref),
                    },
                }
            }
        } else {
            self.system.enqueue_actor(context.actor_ref());
        }

        failsafe.cancel();
    }

    fn kill(&self, actor: Arc<CanReceive>) {
        let mut children = self.children.lock().unwrap();
        let mut index = None;
        for (i, child) in children.iter().enumerate() {
            if child.1.equals(&*actor) {
                index = Some(i);
            }
        }
        for i in index.iter() {
            children.swap_remove(*i);
        }
    }

    fn start(&self, context: ActorCell<Args, M, A>) {
        self.actor.write().unwrap().pre_start(context);
        *self.actor_state.write().unwrap() = ActorState::Running;
    }

    fn restart(&self, context: ActorCell<Args, M, A>) {
        let mut actor = self.actor.write().unwrap();
        actor.pre_restart(context.clone());
        *actor = self.props.create();
        actor.post_restart(context.clone());
        *self.actor_state.write().unwrap() = ActorState::Running;
    }
}

impl<Args: Message, M: Message, A: Actor<M> + 'static> Drop for InnerActorCell<Args, M, A> {
    fn drop(&mut self) {
        // FIXME(gamazeps) Looking at the logs it seems as though fathers are killed before their
        // children, that is not the intended behaviour.
        let actor = self.actor.write().unwrap();
        actor.post_stop();
    }
}
