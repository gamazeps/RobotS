//! Actor system library.

#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

//#![warn(missing_docs,
//        missing_copy_implementations,
//        trivial_casts,
//        trivial_numeric_casts,
//        unused_import_braces,
//        unused_qualifications)]

use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::thread;

pub use self::actors::{Actor, Message};
pub use self::actor_ref::ActorRef;
use self::actor_cell::ActorCell;

pub mod actors;
pub mod actor_ref;
mod actor_cell;
