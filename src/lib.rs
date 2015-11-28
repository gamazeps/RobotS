//! Actor system library.

#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#![warn(missing_docs,
        missing_copy_implementations,
        trivial_casts,
        trivial_numeric_casts,
        unused_import_braces,
        unused_qualifications)]
//#![deny(warnings)]

extern crate eventual;

pub use self::actors::Actor;
pub use self::actor_ref::{ActorRef, CanReceive};
pub use self::actor_system::ActorSystem;
pub use self::props::Props;
pub use self::actor_cell::{ActorCell, ActorContext, SystemMessage};

/// Module for the Actor trait.
pub mod actors;

/// Module for ActorRef, what is used for manipulating Actors.
pub mod actor_ref;

/// Module for the ActorSystem.
pub mod actor_system;

/// Module for Actor factories (Props).
pub mod props;

/// Module with the internals of Actors.
pub mod actor_cell;

/// Module allowing the use of the ask pattern.
pub mod ask;

/// Module containing the original actor.
mod cthulhu;

mod user_actor;
