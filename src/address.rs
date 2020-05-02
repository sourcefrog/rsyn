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
use std::process::{Command, Stdio};

use anyhow::Context;

use crate::{Connection, Options, Result};

/// SSH command name, to start it as a subprocess.
const SSH_COMMAND: &str = "ssh";
/// rsync command name, to start it as a subprocess either locally or remotely.
const RSYNC_COMMAND: &str = "rsync";

/// The address of an rsync server, including
/// information about how to open the connection.
///
/// After building up the desired configuration, use [`.connect()`](#method.connect)
/// to open a [`Connection`](struct.Connection.html) to transfer files.
///
/// Various constructor methods define Addresses of various types.  For example:
/// ```
/// let address = rsyn::Address::local("./src");
/// ```
#[derive(Debug)]
pub struct Address {
    /// Root path to pass to the server.
    path: OsString,

    /// How to start the SSH transport, if applicable.
    ssh: Option<Ssh>,
}

/// Describes how to start an SSH subprocess.
#[derive(Debug)]
struct Ssh {
    user: Option<String>,
    host: String,
}

impl Address {
    /// Builds an Address that, when connected, starts an `rsync --server` subprocess
    /// on the local machine.
    ///
    /// This is primarily useful for testing.
    pub fn local<P: AsRef<Path>>(path: P) -> Address {
        Address {
            path: path.as_ref().as_os_str().into(),
            ssh: None,
        }
    }

    /// Builds the address of an rsync server connected across ssh.
    ///
    /// This will run an external SSH process, defaulting to `ssh`.
    ///
    /// If `user` is None, ssh's default username, typically the same as the
    /// local user, has effect.
    ///
    /// `path` is the path on the remote host to address.
    pub fn ssh(user: Option<&str>, host: &str, path: &str) -> Address {
        Address {
            path: path.into(),
            ssh: Some(Ssh {
                user: user.map(String::from),
                host: host.into(),
            }),
        }
    }

    /// Builds the arguments to start a connection subcommand, including the
    /// command name.
    fn build_args(&self) -> Result<Vec<OsString>> {
        let mut v = Vec::<OsString>::new();
        let mut push_str = |s: &str| v.push(s.into());
        if let Some(ref ssh) = self.ssh {
            push_str(SSH_COMMAND);
            if let Some(ref user) = ssh.user {
                push_str("-l");
                push_str(user);
            }
            push_str(&ssh.host);
            push_str(RSYNC_COMMAND);
        } else {
            push_str(RSYNC_COMMAND);
        };
        push_str("--server");
        push_str("--sender");
        push_str("-vvr");
        v.push(self.path.clone());
        Ok(v)
    }

    /// Opens a connection to this address.
    ///
    /// The `Address` can be opened any number of times, but each `Connection`
    /// can only do a single operation.
    pub fn connect(&self, options: Options) -> Result<Connection> {
        let mut args = self.build_args()?;
        let mut command = Command::new(args.remove(0));
        command.args(args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to launch rsync subprocess {:?}", command))?;

        let r = Box::new(child.stdout.take().expect("Child has no stdout"));
        let w = Box::new(child.stdin.take().expect("Child has no stdin"));

        Connection::handshake(r, w, child, options)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Actually running SSH is a bit hard to test hermetically, but let's
    // at least check the command lines are plausible.

    #[test]
    fn build_local_args() {
        let args = Address::local("./src").build_args().unwrap();
        assert_eq!(args, vec!["rsync", "--server", "--sender", "-vvr", "./src"],);
    }

    #[test]
    fn build_ssh_args() {
        let args = Address::ssh(None, "samba.org", "/home/mbp")
            .build_args()
            .unwrap();
        assert_eq!(
            args,
            vec![
                "ssh",
                "samba.org",
                "rsync",
                "--server",
                "--sender",
                "-vvr",
                "/home/mbp"
            ],
        );
    }

    #[test]
    fn build_ssh_args_with_user() {
        let args = Address::ssh(Some("mbp"), "samba.org", "/home/mbp")
            .build_args()
            .unwrap();
        assert_eq!(
            args,
            vec![
                "ssh",
                "-l",
                "mbp",
                "samba.org",
                "rsync",
                "--server",
                "--sender",
                "-vvr",
                "/home/mbp"
            ],
        );
    }
}
