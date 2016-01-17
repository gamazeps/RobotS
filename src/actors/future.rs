use std::any::Any;
use std::sync::Mutex;

use actors::{Actor, ActorCell, ActorContext, ActorPath, ActorRef};

enum FutureMessages {
    Complete(Box<Any>),
    Calculation(Box<Any>),
    Extract,
}

#[derive(PartialEq)]
enum FutureState {
    Uncompleted,
    Computing,
    Terminated,
    Extracted,
}

struct Future {
    value: Mutex<Option<Box<Any + Send>>>,
    state: Mutex<FutureState>,
}

impl Future {
    fn new() -> Future {
        Future {
            value: Mutex::new(None),
            state: Mutex::new(FutureState::Uncompleted),
        }
    }
}

impl Actor for Future {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        // NOTE: We may want to fail if the message is not correct.
        if let Ok(message) = Box::<Any>::downcast::<FutureMessages>(message) {
            match *message {
                FutureMessages::Complete(msg) => {
                    let mut state = self.state.lock().unwrap();
                    if *state != FutureState::Uncompleted {
                        // NOTE: Send a failure to the sender instead.
                        panic!("Tried to complete a Future twice");
                    }
                    *self.value.lock().unwrap() = Some(msg);
                    *state = FutureState::Computing;
                },
                FutureMessages::Calculation(msg) => {
                    // FIXME(gamazeps): check the state.
                    if let Ok(func) = Box::<Any>::downcast::<Fn(Box<Any>, ActorCell) -> Option<Box<Any>>>(message) {
                        let mut value = self.value.lock().unwrap();
                        let v = value.take().unwrap();
                        *value = (*func)(v, context);
                        if *value == None {
                            *self.state.lock().unwrap() = FutureState::Terminated;
                            context.kill_me();
                        }
                    }

                },
                FutureMessages::Extract => {
                    let mut value = self.value.lock().unwrap();
                    let v = value.take();
                    *value = None;
                    *self.state.lock().unwrap() = FutureState::Extracted;
                    context.tell(context.sender(), v);
                    context.kill_me()
                },
            }
        }
    }
}
