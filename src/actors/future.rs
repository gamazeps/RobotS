use std::any::Any;
use std::mem;
use std::sync::{Arc, Mutex};

use actors::{Actor, ActorCell, ActorContext, ActorPath, ActorRef, Message};

macro_rules! complete {
    ($future:expr, $complete:expr, $context:expr) => {
        $context.tell($future, FutureMessages::Complete(Box::new($complete) as Box::<Any + Send>));
    }
}

macro_rules! do_calculation {
    ($future:expr, $calculation:expr, $context:expr) => {
        $context.tell($future, FutureMessages::Complete(Box::new($complete) as Box::<Any + Send>));
    }
}


#[derive(Clone)]
pub enum FutureMessages {
    /// We complete the future with the value inside the enum.
    Complete(Arc<Any + Send + Sync>),
    /// We apply the following closure to the value inside the Future and update it with the
    /// result.
    ///
    /// *  Extracted will extract the result from the future and kill it.
    /// *  NewValue will update the value inside the Future.
    /// *  Done will kill the Future after the calculations are done.
    ///
    /// Note that Done and Extracted might be a double of each other, I'll try to remove it
    /// afterwards.
    Calculation(Arc<Fn(Box<Any>, ActorCell) -> FutureState + Send + Sync>),
}

pub enum FutureState {
    Uncompleted,
    Computing(Box<Any + Send>),
    Terminated,
    Extracted,
}

pub struct Future {
    state: Mutex<Option<FutureState>>,
}

impl Future {
    pub fn new(_dummy: ()) -> Future {
        Future {
            state: Mutex::new(Some(FutureState::Uncompleted)),
        }
    }
}

trait LocalShit: Any + Send {}
impl<T> LocalShit for T where T: Any + Send {}

impl Actor for Future {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        // NOTE: We may want to fail if the message is not correct.
        if let Ok(message) = Box::<Any>::downcast::<FutureMessages>(message) {
            match *message {
                FutureMessages::Complete(mut msg) => {
                    let mut state = self.state.lock().unwrap();
                    let s = state.take().unwrap();
                    match s {
                        FutureState::Uncompleted => {
                            *state = Some(FutureState::Computing(unsafe {
                                let msg = Arc::get_mut(&mut msg).unwrap();
                                Box::<Any + Send>::from_raw(msg)
                            }));
                        },
                        _ => {
                            // NOTE: Send a failure to the sender instead.
                            panic!("Tried to complete a Future twice");
                        }
                    }
                },
                FutureMessages::Calculation(func) => {
                    // FIXME(gamazeps): check the state.
                    let mut state = self.state.lock().unwrap();
                    let s = state.take().unwrap();
                    match s {
                        FutureState::Computing(value) => {
                            let res = (*func)(value, context.clone());
                            match res {
                                FutureState::Computing(v) => *state = Some(FutureState::Computing(v)),
                                FutureState::Terminated => {
                                    *state = Some(FutureState::Terminated);
                                    context.kill_me();
                                }
                                FutureState::Extracted => {
                                    *state = Some(FutureState::Extracted);
                                    context.kill_me();}
                                ,
                                FutureState::Uncompleted => {
                                    *state = Some(FutureState::Uncompleted);
                                    panic!("A future closure returned Uncompleted, this should not happen");
                                },
                            }
                        },
                        FutureState::Uncompleted => panic!("A closure was called on an uncompleted Future."),
                        FutureState::Terminated => panic!("A closure was called on a Terminated Future."),
                        FutureState::Extracted => panic!("A closure was called on an extracted Future."),
                    }
                },
            }
        }
    }
}
