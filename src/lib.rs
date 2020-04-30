//! A wire-compatible rsync client in Rust.
//!
//! Use the `Connection` type to open a connection then list or transfer files:
//!
//! ```
//! // Open a connection to a local rsync server, and list the source directory.
//! let flist = rsyn::Connection::local_subprocess("./src").unwrap()
//!     .list_files().unwrap();
//!
//! // We can see the `lib.rs` in the listing.
//! assert!(flist.iter().any(|fe|
//!     String::from_utf8_lossy(&fe.name).ends_with("lib.rs")));
//! ```

mod connection;
mod flist;
pub mod logging;
mod mux;
mod statistics;
mod varint;

pub use connection::Connection;
pub use flist::{FileEntry, FileList};
pub use statistics::ServerStatistics;

/// General Result type from rsyn APIs.
pub type Result<T> = anyhow::Result<T>;
