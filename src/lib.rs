// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A wire-compatible rsync client in Rust.
//!
//! Use the [`Address`](struct.Address.html) type to list or transfer files:
//!
//! ```
//! use rsyn::{Address, Options};
//! let address = Address::local("./src");
//! // Open a connection to a local rsync server, and list the source directory.
//! let options = Options {
//!     list_only: true,
//!     recursive: true,
//!     ..Options::default()
//! };
//! let (flist, _stats) = address.list_files(options)?;
//!
//! // We can see the `lib.rs` in the listing.
//! assert!(flist.iter().any(|fe|
//!     fe.name_lossy_string().ends_with("lib.rs")));
//! # rsyn::Result::Ok(())
//! ```

mod address;
mod connection;
mod flist;
pub mod logging;
mod mux;
mod options;
mod statistics;
mod varint;

pub use address::Address;
pub use connection::Connection;
pub use flist::{FileEntry, FileList};
pub use options::Options;
pub use statistics::ServerStatistics;

/// General Result type from rsyn APIs.
pub type Result<T> = anyhow::Result<T>;
