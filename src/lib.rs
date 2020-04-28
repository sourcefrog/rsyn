//! Library for wire-compatible rsync client in Rust.

mod connection;
mod flist;
pub mod logging;
mod mux;
mod statistics;
mod varint;

pub use connection::Connection;

/// General Result type from rsyn APIs.
pub use anyhow::Result;
