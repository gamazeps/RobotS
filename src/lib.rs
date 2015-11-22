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

pub use self::actors::{Actor, Message};
pub use self::actor_ref::{ActorRef, CanReceive};
pub use self::actor_system::ActorSystem;
pub use self::props::Props;
use self::actor_cell::ActorCell;

pub mod actors;
pub mod actor_ref;
pub mod actor_system;
pub mod props;
mod actor_cell;
