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

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(future_incompatible)]
#![warn(rust_2018_idioms)]
// private_doc_tests is a nice idea but unfortunately warns on types republished
// by `pub use`.
// https://github.com/rust-lang/rust/issues/72081
#![allow(private_doc_tests)]
// MAYBE: warn(missing-doc-code-examples) but covering everything isn't a
// priority yet.
#![warn(intra_doc_link_resolution_failure)]
// Match on Ord isn't any easier to read.
#![allow(clippy::comparison_chain)]

//! A wire-compatible rsync client in Rust.
//!
//! Messages are sent to [`log`](https://docs.rs/log/) and a log destination
//! may optionally be configured by clients.
//!
//! Use the [`Client`](struct.Client.html) type to list or transfer files:
//!
//! ```
//! // Open a connection to a local rsync server, and list the source directory.
//! use rsyn::{Client, Options};
//!
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
mod mux;
mod options;
mod statistics;
mod sums;
mod varint;

pub use client::Client;
pub use flist::{FileEntry, FileList};
pub use options::Options;
pub use statistics::ServerStatistics;

/// General Result type from rsyn APIs.
pub type Result<T> = anyhow::Result<T>;

const MD4_SUM_LENGTH: usize = 16;
