/// This module contains most of the internals of actors.
///
/// It is used to handle messages, system messages, termination, initialization, restarting and
/// creation of actors.

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::{Arc, Mutex, RwLock, Weak};

use actors::{Actor, ActorPath, ActorRef, ActorSystem, Message, Props};
use actors::future::{Computation, Complete, Future, FutureState};
use actors::name_resolver::ResolveRequest;
use actors::props::ActorFactory;

/// Closure to handle failure of an Actor.
pub type FailureHandler = Arc<Fn(ActorRef, ActorCell) + Send + Sync>;

enum Ref<T: ?Sized> {
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

/// Main interface for interractiong with an Actor for the internals.
pub struct ActorCell {
    // We have an inner structure in order to be able to generate new ActorCell easily.
    inner_cell: Ref<InnerActorCell>,
}

impl Clone for ActorCell {
    fn clone(&self) -> ActorCell {
        ActorCell {
            inner_cell: Ref::WeakRef(match self.inner_cell {
                Ref::StrongRef(ref inner) => Arc::downgrade(&inner),
                Ref::WeakRef(ref inner) => inner.clone(),
            }),
        }
    }
}


impl ActorCell {
    /// Creates a new ActorCell.
    pub fn new( props: Arc<ActorFactory>,
               system: ActorSystem,
               father: ActorRef,
               path: Arc<ActorPath>)
               -> ActorCell {
        ActorCell {
            inner_cell: Ref::StrongRef(Arc::new(InnerActorCell::new(props,
                                                                    system,
                                                                    father,
                                                                    path))),
        }
    }

    /// Puts a message with its sender in the Actor's mailbox and schedules the Actor.
    pub fn receive_message(&self, message: InnerMessage, sender: ActorRef) {
        let inner = unwrap_inner!(self.inner_cell, {
            warn!("A message was send to a ref to a stopped actor");
            return;
        });
        inner.receive_message(message, sender);
        inner.system.enqueue_actor(self.actor_ref());
    }

    /// Puts a system message with its sender in the Actor's system mailbox and schedules the Actor.
    pub fn receive_system_message(&self, system_message: SystemMessage) {
        let inner = unwrap_inner!(self.inner_cell, {
            warn!("A message was send to a ref to a stopped actor");
            return;
        });
        inner.receive_system_message(system_message);
        inner.system.enqueue_actor(self.actor_ref());
    }

    /// Makes the Actor handle an envelope in its mailbox.
    pub fn handle_envelope(&self) {
        let inner = unwrap_inner!(self.inner_cell, {
            warn!("A message was send to a ref to a stopped actor");
            return;
        });
        inner.handle_envelope(self.clone());
    }
}

/// This is the API that Actors are supposed to see of their context while handling a message.
pub trait ActorContext {
    /// Returns an ActorRef to the Actor.
    fn actor_ref(&self) -> ActorRef;

    /// Spawns a child actor.
    fn actor_of(&self, props: Arc<ActorFactory>, name: String) -> Result<ActorRef, &'static str>;

    /// Sends a Message to the targeted ActorRef.
    fn tell<MessageTo: Message>(&self, to: ActorRef, message: MessageTo);

    /// Creates a Future, this Future will send the message to the targetted ActorRef (and thus be
    /// the sender of the message).
    fn ask<MessageTo: Message>(&self, to: ActorRef, message: MessageTo, future_name: String) -> ActorRef;

    /// Completes a Future.
    fn complete<MessageTo: Message>(&self, to: ActorRef, complete: MessageTo);

    /// Tells a future to forward its result to another Actor.
    /// The Future is then dropped.
    fn forward_result<T: Message>(&self, future: ActorRef, to: ActorRef);

    /// Tells a future to forward its result to another Future that will be completed with this
    /// result.
    /// The Future is then dropped.
    fn forward_result_to_future<T: Message>(&self, future: ActorRef, to: ActorRef);

    /// Sends the Future a closure to apply on its value, the value will be updated with the output
    /// of the closure.
    fn do_computation<T: Message, F: Fn(Box<Any + Send>, ActorCell) -> T + Send + Sync + 'static>
        (&self, future: ActorRef, closure: F);

    /// Requests the targeted actor to stop.
    fn stop(&self, actor_ref: ActorRef);

    /// Asks the father of the actor to terminate it.
    fn kill_me(&self);

    /// Returns an Arc to the sender of the message being handled.
    fn sender(&self) -> ActorRef;

    /// Father of the actor.
    fn father(&self) -> ActorRef;

    /// Children of the actor.
    fn children(&self) -> HashMap<Arc<ActorPath>, ActorRef>;

    /// Lifecycle monitoring, list of monitored actors.
    fn monitoring(&self) -> HashMap<Arc<ActorPath>, (ActorRef, FailureHandler)>;

    /// Logical path to the actor, such as `/user/foo/bar/baz`
    fn path(&self) -> Arc<ActorPath>;

    /// Future containing an Option<ActorRef> with an ActtorRef to the Actor with the given logical
    /// path.
    ///
    /// The future will have the path: `$actor/$name_request`
    fn identify_actor(&self, logical_path: String, request_name: String) -> ActorRef;

    ///// Monitors the given actor and will treat him with the given handler.
    //fn monitor(&self, actor: ActorRef, handler: FailureHandler);
}

impl ActorContext for ActorCell {
    fn actor_ref(&self) -> ActorRef {
        ActorRef::with_cell(self.clone(), self.path())
    }

    fn actor_of(&self, props: Arc<ActorFactory>, name: String) -> Result<ActorRef, &'static str> {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to create an actor from the context of a no longer existing actor");
        });

        // We check that there is no path traversal.
        if name.find("/") != None {
            return Err("Used a '/' in the name of an actor, this is not allowed");
        }

        let path = self.path().child(name);
        info!("creating actor {}", path.logical_path());
        let inner_cell = InnerActorCell::new(props,
                                             inner.system.clone(),
                                             self.actor_ref(),
                                             path.clone());
        let actor_cell = ActorCell { inner_cell: Ref::StrongRef(Arc::new(inner_cell)) };
        let internal_ref = ActorRef::with_cell(actor_cell, path.clone());
        let external_ref = internal_ref.clone();
        inner.children.lock().unwrap().insert(path.clone(), internal_ref);
        inner.monitoring.lock().unwrap().insert(path.clone(), (external_ref.clone(), Arc::new(InnerActorCell::restart_child)));
        external_ref.receive_system_message(SystemMessage::Start);
        // This is a bit messy, but we have a chicken / egg issue otherwise when creating the name
        // resolver actor.
        if *(path.logical_path()) != "/system/name_resolver" {
            self.tell(inner.system.name_resolver(), ResolveRequest::Add(external_ref.clone()));
        }
        Ok(external_ref)
    }

    fn tell<MessageTo: Message>(&self, to: ActorRef, message: MessageTo) {
        // FIXME(gamazeps): Code duplication.
        let path = to.path();
        match *path {
            ActorPath::Local(_) => to.receive(InnerMessage::Message(Box::new(message)), self.actor_ref()),
            ActorPath::Distant(ref path) => {
                info!("Sent a message of size {} to distant actor {}:{}", mem::size_of::<MessageTo>(),
                path.distant_logical_path(), path.addr_port());
            },
        }
    }

    fn ask<MessageTo: Message>(&self, to: ActorRef, message: MessageTo, name: String) -> ActorRef {
        let future = self.actor_of(Props::new(Arc::new(Future::new), ()), name).unwrap();
        future.tell_to(to, message);
        future
    }

    fn complete<MessageTo: Message>(&self, future: ActorRef, complete: MessageTo) {
        // FIXME(gamazeps): Code duplication.
        // This is a copy of the code in tell, but we need to do that in order to put a Box<Any> in
        // the mailbox.
        let path = future.path();
        match *path {
            ActorPath::Local(_) => future.receive(InnerMessage::Message(Box::new(Complete::new(Box::new(complete)))), self.actor_ref()),
            ActorPath::Distant(ref path) => {
                info!("Sent a message of size {} to distant future {}:{}", mem::size_of::<MessageTo>(),
                path.distant_logical_path(), path.addr_port());
            },
        }
    }

    fn forward_result<T: Message>(&self, future: ActorRef, actor: ActorRef) {
        self.tell(future, Computation::Forward(actor, Arc::new(move |value, context, to| {
            // FIXME(gamazeps): error handling for cthulhu's sake !
            if let Ok(value) = Box::<Any + Send>::downcast::<T>(value) {
                context.tell(to, *value);
            }
            FutureState::Extracted
        })));
    }

    fn forward_result_to_future<T: Message>(&self, future: ActorRef, actor: ActorRef) {
        self.tell(future, Computation::Forward(actor, Arc::new(move |value, context, to| {
            // FIXME(gamazeps): error handling for cthulhu's sake !
            if let Ok(value) = Box::<Any + Send>::downcast::<T>(value) {
                context.complete(to, *value);
            }
            FutureState::Extracted
        })));
    }

    fn do_computation<T: Message, F: Fn(Box<Any + Send>, ActorCell) -> T + Send + Sync + 'static>
        (&self, future: ActorRef, closure: F) {
        self.tell(future, Computation::Computation(Arc::new(move |value, context| {
            let v = closure(value, context);
            FutureState::Computing(Box::new(v))
        })));
    }

    fn sender(&self) -> ActorRef {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to get a sender from the context of a no longer existing actor");
        });
        // This is weird but this is for clippy.
        let current_sender = inner.current_sender.lock().unwrap();
        current_sender.as_ref().unwrap().clone()
    }

    fn stop(&self, actor_ref: ActorRef) {
        actor_ref.receive(InnerMessage::Control(ControlMessage::PoisonPill),
                          self.actor_ref());
    }

    fn kill_me(&self) {
        self.father().receive(InnerMessage::Control(ControlMessage::KillMe(self.actor_ref())),
                              self.actor_ref());
    }

    fn father(&self) -> ActorRef {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to get the father from the context of a no longer existing actor");
        });
        inner.father.clone()
    }

    fn children(&self) -> HashMap<Arc<ActorPath>, ActorRef> {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to get the children from the context of a no longer existing actor");
        });
        let children = inner.children.lock().unwrap();
        children.clone()
    }

    fn monitoring(&self) -> HashMap<Arc<ActorPath>, (ActorRef, FailureHandler)> {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to get the monitored actors from the context of a no longer existing \
                    actor");
        });
        let monitoring = inner.monitoring.lock().unwrap();
        monitoring.clone()
    }

    fn path(&self) -> Arc<ActorPath> {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to get the path from the context of a no longer existing actor");
        });
        inner.path.clone()
    }

    fn identify_actor(&self, name: String, request_name: String) -> ActorRef {
        let inner = unwrap_inner!(self.inner_cell, {
            panic!("Tried to get the actor system of a no longer existing actor while resolving \
                    a path. This should *never* happen");
        });
        self.ask(inner.system.name_resolver(), ResolveRequest::Get(name), request_name)
    }
}

#[derive(PartialEq)]
/// Interna representation of the actor's state.
enum ActorState {
    /// The actor has panicked and has not yet been restarded.
    Failed,
    /// The actor is up and running.
    Running,
    /// The actor is in a clean state, but has not initiazed itself yet.
    Unstarted,
}

/// Structure used to send a failure message when the actor panics.
struct Failsafe {
    context: ActorCell,
    state: Arc<RwLock<ActorState>>,
    active: bool,
}

impl Failsafe {
    fn new(context: ActorCell, state: Arc<RwLock<ActorState>>) -> Failsafe {
        Failsafe {
            context: context,
            state: state,
            active: true,
        }
    }

    /// Cancels the failsafe, means that everything went normally.
    fn cancel(mut self) {
        self.active = false;
    }
}

impl Drop for Failsafe {
    fn drop(&mut self) {
        if self.active {
            *self.state.write().unwrap() = ActorState::Failed;
            self.context.father().receive_system_message(SystemMessage::Failure(self.context.actor_ref()));
        }
    }
}

/// Special messages issued by the actor system.
/// Note that these are treated with the highest priority and will thus be handled before any
/// InnerMessage is handled.
#[derive(Clone)]
pub enum SystemMessage {
    /// Restarts the actor by replacing it with a new version created with its ActorFactory.
    Restart,

    /// Tells the actor to initialize itself.
    /// Note that the initialization is not done by the father for fairness reasons.
    Start,

    /// Tells an actor that its child failed.
    Failure(ActorRef),
}

/// Structure used to store a message and its sender.
struct Envelope {
    message: InnerMessage,
    sender: ActorRef,
}

/// Types of message that can be sent to an actor that will be treated normally.
pub enum InnerMessage {
    /// Regular message.
    Message(Box<Any + Send>),

    /// Control messages.
    Control(ControlMessage),
}

/// Control Messages.
#[derive(Clone)]
pub enum ControlMessage {
    /// Requests the termination of the actor.
    /// This is what is sent when the `context.stop(actor_ref)` is called.
    PoisonPill,

    /// Message sent to the monitoring actors when the actor is terminated.
    Terminated(ActorRef),

    /// Message sent to the father of an actor to request being terminated.
    KillMe(ActorRef),
}

struct InnerActorCell {
    mailbox: Mutex<VecDeque<Envelope>>,
    system_mailbox: Mutex<VecDeque<SystemMessage>>,
    props: Arc<ActorFactory>,
    system: ActorSystem,
    path: Arc<ActorPath>,
    current_sender: Mutex<Option<ActorRef>>,
    busy: Mutex<()>,
    father: ActorRef,
    children: Mutex<HashMap<Arc<ActorPath>, ActorRef>>,
    monitoring: Mutex<HashMap<Arc<ActorPath>, (ActorRef, FailureHandler)>>,
    actor_state: Arc<RwLock<ActorState>>,
    _monitored: Mutex<Vec<ActorRef>>,
    actor: RwLock<Arc<Actor>>,
}

impl InnerActorCell {
    /// Constructor.
    fn new(props: Arc<ActorFactory>,
           system: ActorSystem,
           father: ActorRef,
           path: Arc<ActorPath>)
           -> InnerActorCell {
        InnerActorCell {
            actor: RwLock::new(props.create()),
            mailbox: Mutex::new(VecDeque::new()),
            system_mailbox: Mutex::new(VecDeque::new()),
            props: props,
            system: system,
            path: path,
            current_sender: Mutex::new(None),
            busy: Mutex::new(()),
            father: father.clone(),
            children: Mutex::new(HashMap::new()),
            monitoring: Mutex::new(HashMap::new()),
            actor_state: Arc::new(RwLock::new(ActorState::Unstarted)),
            _monitored: Mutex::new(vec![father.clone()]),
        }
    }

    fn receive_envelope(&self, envelope: Envelope) {
        self.mailbox.lock().unwrap().push_back(envelope);
    }

    fn receive_message(&self, message: InnerMessage, sender: ActorRef) {
        self.receive_envelope(Envelope {
            message: message,
            sender: sender,
        });
    }

    fn receive_system_message(&self, system_message: SystemMessage) {
        self.system_mailbox.lock().unwrap().push_back(system_message);
    }

    fn handle_envelope(&self, context: ActorCell) {
        // Now we do not want users to be able to touch current_sender while the actor is busy.
        let _lock = self.busy.lock();
        let failsafe = Failsafe::new(context.clone(),
                                     self.actor_state.clone());
        // System messages are handled first, so that we can restart an actor if he failed without
        // loosing the messages in the mailbox.
        // NOTE: This does not break the fact that messages sent by the same actor are treated in
        // the order they are sent (if all to the same target actor), as system messages must not
        // be sent by other actors by the user.
        if let Some(message) = self.system_mailbox.lock().unwrap().pop_front() {
            match message {
                SystemMessage::Restart => self.restart(context),
                SystemMessage::Start => self.start(context),
                SystemMessage::Failure(actor) => {
                    let monitoring = self.monitoring.lock().unwrap();
                    let handler = monitoring.get(&actor.path())
                        .expect("Received a failure notification from an unknown actor");
                    (*handler.1)(actor, context);
                }
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
                    InnerMessage::Message(message) => {
                        actor.receive(message, context);
                    },
                    InnerMessage::Control(message) => {
                        match message {
                            ControlMessage::PoisonPill => context.kill_me(),
                            ControlMessage::Terminated(_) => actor.receive_termination(context),
                            ControlMessage::KillMe(actor_ref) => self.kill(actor_ref, context),
                        }
                    }
                }
            }
        } else {
            self.system.enqueue_actor(context.actor_ref());
        }

        failsafe.cancel();
    }

    fn kill(&self, actor: ActorRef, context: ActorCell) {
        self.children.lock().unwrap().remove(&actor.path()).expect(&format!("actor {} was asked to kill {} and cannot do that",
                                             self.path.logical_path(),
                                             actor.path().logical_path()));
        context.tell(self.system.name_resolver(), ResolveRequest::Remove(actor.path()));
    }

    fn start(&self, context: ActorCell) {
        self.actor.write().unwrap().pre_start(context);
        *self.actor_state.write().unwrap() = ActorState::Running;
    }

    fn restart(&self, context: ActorCell) {
        let mut actor = self.actor.write().unwrap();
        actor.pre_restart(context.clone());
        *actor = self.props.create();
        actor.post_restart(context);
        *self.actor_state.write().unwrap() = ActorState::Running;
    }

    fn restart_child(actor: ActorRef, _context: ActorCell) {
        actor.receive_system_message(SystemMessage::Restart);
    }
}

impl Drop for InnerActorCell {
    fn drop(&mut self) {
        // FIXME(gamazeps) Looking at the logs it seems as though fathers are killed before their
        // children, that is not the intended behaviour.
        let actor = self.actor.write().unwrap();
        info!("Actor {} is dropped", *self.path.logical_path());
        actor.post_stop();
    }
}
