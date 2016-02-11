use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use actors::{Actor, ActorCell, ActorContext, ActorRef, Message};


pub struct Complete {
    complete: Box<Any + Send>,
}

impl Complete {
    pub fn new(complete: Box<Any + Send>) -> Complete {
        Complete {
            complete: complete,
        }
    }
}

#[derive(Clone)]
pub enum Computation {
    Forward(ActorRef, Arc<Fn(Box<Any + Send>, ActorCell, ActorRef) -> FutureState + Send + Sync>),
    // This is a terrible name.
    Computation(Arc<Fn(Box<Any + Send>, ActorCell) -> FutureState + Send + Sync>),
}

pub enum FutureState {
    Uncompleted,
    Computing(Box<Any + Send>),
    Extracted,
}

pub struct Future {
    state: Mutex<Option<FutureState>>,
    scheduled_calculations: Mutex<VecDeque<Computation>>,
}

impl Future {
    pub fn new(_dummy: ()) -> Future {
        Future {
            state: Mutex::new(Some(FutureState::Uncompleted)),
            scheduled_calculations: Mutex::new(VecDeque::new()),
        }
    }

    fn handle_computation(&self, computation: Computation, context: ActorCell) {
        info!("{} handling a computation", context.actor_ref().path().logical_path());
        let mut state = self.state.lock().unwrap();
        let s = state.take().unwrap();
        match s {
            FutureState::Computing(value) => {
                match computation {
                    Computation::Forward(to, func) => {
                        info!("{} forwarding to {}", context.actor_ref().path().logical_path(), to.path().logical_path());
                        *state = Some((*func)(value, context.clone(), to));
                    },
                    Computation::Computation(func) => {
                        *state = Some((*func)(value, context.clone()));
                    }
                }
                match *state {
                    Some(FutureState::Computing(_)) => {},
                    Some(FutureState::Extracted) => context.kill_me(),
                    Some(FutureState::Uncompleted) => panic!("A future closure returned Uncompleted, this should not happen"),
                    None => unreachable!(),
                }
            }
            // In the next cases we put the state back to how it was.
            FutureState::Uncompleted => {
                *state = Some(s);
                info!("{} is keeping the computation for later", context.actor_ref().path().logical_path());
                self.scheduled_calculations.lock().unwrap().push_back(computation);
            },
            FutureState::Extracted => {
                *state = Some(s);
                panic!("A closure was called on an Extracted Future.");
            },
        }
    }
}

impl Actor for Future {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        match Box::<Any>::downcast::<Computation>(message) {
            Ok(computation) => {
                self.handle_computation(*computation, context);
            },
            Err(message) => {
                // The double downgrade is ugly but we can't have an enum containing the two
                // variants (see http://gamazeps.github.io/notes-7.html).
                if let Ok(msg) = Box::<Any>::downcast::<Complete>(message) {
                    // We need to free the lock on the state.
                    {
                        let mut state = self.state.lock().unwrap();
                        let s = state.take().unwrap();
                        match s {
                            FutureState::Uncompleted => {
                                // We complete the future with the received value, an unsafe cast is
                                // nevertheless needed to get a Box<Any + Send>
                                *state = Some(FutureState::Computing((*msg).complete));
                                info!("{} has been completed", context.actor_ref().path().logical_path());
                            },
                            _ => {
                                // NOTE: Send a failure to the sender instead.
                                panic!("Tried to complete a {} twice", context.actor_ref().path().logical_path());
                            },
                        }
                    }

                    // We now do the previous scheduled computations.
                    let mut scheduled_calculations = self.scheduled_calculations.lock().unwrap();
                    while let Some(func) = scheduled_calculations.pop_front() {
                        self.handle_computation(func, context.clone());
                    }
                } else {
                    info!("{} received a message of the wrong format from {}", context.actor_ref().path().logical_path(),
                    context.sender().path().logical_path());
                }
            }
        }
    }
}

pub struct FutureExtractor<T: Message> {
    future: ActorRef,
    channel: Arc<Mutex<Sender<T>>>,
}

impl<T: Message> FutureExtractor<T> {
    pub fn new(args: (ActorRef, Arc<Mutex<Sender<T>>>)) -> FutureExtractor<T> {
        FutureExtractor {
            future: args.0,
            channel: args.1,
        }
    }
}

impl<T: Message> Actor for FutureExtractor<T> {
    // Here when the extractor is created it tells the future to forward it its result.
    fn pre_start(&self, context: ActorCell) {
        context.forward_result::<T>(self.future.clone(), context.actor_ref());
    }

    // It then receives the result and will send it through its channel.
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<T>(message) {
            info!("The extractor {} received the type it wants to extract", context.actor_ref().path().logical_path());
            // FIXME(gamazeps): error handling.
            let _res = self.channel.lock().unwrap().send(*message);
            // Once we have sent the message through the channel, we want this actor to be dropped.
            context.kill_me();
        }
    }
}
