/// Library for wire-compatible rsync client in Rust.

mod connection;
mod flist;
mod mux;
mod varint;
mod statistics;

pub use connection::Connection;
