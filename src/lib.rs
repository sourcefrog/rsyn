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

#![warn(missing_docs)]
#![warn(future_incompatible)]
#![warn(rust_2018_idioms)]
#![warn(rustdoc)]

//! A wire-compatible rsync client in Rust.
//!
//! Use the [`Client`](struct.Client.html) type to list or transfer files:
//!
//! ```
//! // Open a connection to a local rsync server, and list the source directory.
//! use rsyn::{Client, Options};
//! let mut client = Client::local("./src");
//! client.set_recursive(true);
//! let (flist, _stats) = client.list_files()?;
//!
//! // We can see the `lib.rs` in the listing.
//! assert!(flist.iter().any(|fe|
//!     fe.name_lossy_string().ends_with("lib.rs")));
//! # rsyn::Result::Ok(())
//! ```

mod client;
mod connection;
mod flist;
pub mod logging;
mod mux;
mod options;
mod statistics;
mod varint;

pub use client::Client;
pub use flist::{FileEntry, FileList};
pub use options::Options;
pub use statistics::ServerStatistics;

/// General Result type from rsyn APIs.
pub type Result<T> = anyhow::Result<T>;
