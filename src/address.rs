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

//! Build the address of an rsync server to connect to.
//!
//! This is the starting point for doing anything else with the library.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use anyhow::Context;

use crate::{Connection, Result};

/// The address of an rsync server to which you can connect, including
/// information about how to open the connection.
///
/// After building up the desired configuration, use `.connect()` to open a
/// `Connection` to transfer files.
#[derive(Debug)]
pub struct Address {
    /// Root path to pass to the server.
    path: OsString,
}

impl Address {
    /// An Address that, when connected, starts an `rsync --server` subprocess
    /// on the local machine.
    ///
    /// This is primarily useful for testing.
    pub fn local<P: AsRef<Path>>(path: P) -> Address {
        Address {
            path: path.as_ref().as_os_str().into(),
        }
    }

    // Generates the subprocess command to run.

    /// Open a connection to this address.
    ///
    /// The `Address` can be opened any number of times, but each `Connection`
    /// can only do a single operation.
    pub fn connect(&self) -> Result<Connection> {
        let mut child: Child = Command::new("rsync")
            .arg("--server")
            .arg("--sender")
            .arg("-vvr")
            .arg(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to launch rsync subprocess")?;

        let r = Box::new(child.stdout.take().expect("child has no stdout"));
        let w = Box::new(child.stdin.take().expect("child has no stdin"));

        Connection::handshake(r, w, child)
    }
}
