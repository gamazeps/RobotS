//! Actor system library.

#![cfg_attr(feature="clippy", feature(plugin))]

#![cfg_attr(feature="clippy", plugin(clippy))]

#![warn(missing_docs,
        missing_copy_implementations,
        trivial_casts,
        trivial_numeric_casts,
        unused_import_braces,
        unused_qualifications)]

#![deny(warnings)]

#[macro_use]
extern crate log;

/// Actors core.
pub mod actors;
