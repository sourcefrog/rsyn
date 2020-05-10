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
use std::str::FromStr;

use anyhow::Context;
use lazy_static::lazy_static;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use regex::Regex;

use crate::connection::Connection;
use crate::{FileList, Options, Result, ServerStatistics};

/// SSH command name, to start it as a subprocess.
const DEFAULT_SSH_COMMAND: &str = "ssh";
/// rsync command name, to start it as a subprocess either locally or remotely.
const DEFAULT_RSYNC_COMMAND: &str = "rsync";

/// The address of an rsync server, including
/// information about how to open the connection.
///
/// Addresses can be parsed from strings:
/// ```
/// use std::str::FromStr;
/// let address = rsyn::Client::from_str("rsync.example.com::module")
///     .expect("Parse failed");
/// ```
///
/// Or constructed:
/// ```
/// let address = rsyn::Client::local("./src");
/// let address = rsyn::Client::ssh(Some("user"), "host.example.com", "./src");
/// ```
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Client {
    /// Root path to pass to the server.
    path: OsString,

    /// How to start the SSH transport, if applicable.
    ssh: Option<Ssh>,

    /// Use the rsync daemon wrapper protocol.
    ///
    /// This can be done either over bare TCP, or wrapped in SSH.
    /// (See "USING RSYNC-DAEMON FEATURES VIA A REMOTE-SHELL CONNECTION" in the
    /// rsync manual.)
    daemon: Option<Daemon>,

    /// Protocol / remote command line options.
    options: Options,
}

#[derive(Clone, Eq, PartialEq, Debug)]
struct Daemon {
    user: Option<String>,
    host: String,
    port: Option<u16>,
}

/// Describes how to start an SSH subprocess.
#[derive(Clone, Eq, PartialEq, Debug)]
struct Ssh {
    user: Option<String>,
    host: String,
}

impl Client {
    /// Builds an Client that, when connected, starts an `rsync --server` subprocess
    /// on the local machine.
    ///
    /// This is primarily useful for testing.
    pub fn local<P: AsRef<Path>>(path: P) -> Client {
        Client {
            path: path.as_ref().as_os_str().into(),
            ssh: None,
            daemon: None,
            options: Options::default(),
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
    pub fn ssh(user: Option<&str>, host: &str, path: &str) -> Client {
        Client {
            path: path.into(),
            ssh: Some(Ssh {
                user: user.map(String::from),
                host: host.into(),
            }),
            daemon: None,
            options: Options::default(),
        }
    }

    /// Mutably borrow this client's `Options`.
    pub fn mut_options(&mut self) -> &mut Options {
        &mut self.options
    }

    /// Replace this client's `Options`.
    pub fn set_options(&mut self, options: Options) -> &mut Self {
        self.options = options;
        self
    }

    /// Set the `recursive` option.
    pub fn set_recursive(&mut self, recursive: bool) -> &mut Self {
        self.options.recursive = recursive;
        self
    }

    /// Set the `verbose` option.
    pub fn set_verbose(&mut self, verbose: u32) -> &mut Self {
        self.options.verbose = verbose;
        self
    }

    /// Builds the arguments to start a connection subcommand, including the
    /// command name.
    fn build_args(&self) -> Vec<OsString> {
        let mut v = Vec::<OsString>::new();
        let mut push_str = |s: &str| v.push(s.into());
        if let Some(ref ssh) = self.ssh {
            if let Some(args) = &self.options.ssh_command {
                for arg in args {
                    push_str(arg)
                }
            } else {
                push_str(DEFAULT_SSH_COMMAND)
            }
            if let Some(ref user) = ssh.user {
                push_str("-l");
                push_str(user);
            }
            push_str(&ssh.host);
        };
        if let Some(rsync_command) = &self.options.rsync_command {
            for arg in rsync_command {
                push_str(arg)
            }
        } else {
            push_str(DEFAULT_RSYNC_COMMAND)
        }
        push_str("--server");
        push_str("--sender");
        if self.options.verbose > 0 {
            let mut o = "-".to_string();
            for _ in 0..self.options.verbose {
                o.push('v');
            }
            push_str(&o);
        }
        if self.options.list_only {
            push_str("--list-only")
        }
        if self.options.recursive {
            push_str("-r")
        }
        if self.path.is_empty() {
            push_str(".")
        } else {
            v.push(self.path.clone())
        }
        v
    }

    /// List files from the remote server.
    ///
    /// This implicitly sets the `list_only` option.
    pub fn list_files(&mut self) -> Result<(FileList, ServerStatistics)> {
        self.connect()
            .context("Failed to connect")?
            .list_files()
            .context("Failed to list files")
    }

    /// Opens a connection to this address.
    ///
    /// The `Client` can be opened any number of times, but each `Connection`
    /// can only do a single operation.
    fn connect(&self) -> Result<Connection> {
        if self.daemon.is_some() {
            todo!("daemon mode is not implemented yet");
        }
        let mut args = self.build_args();
        info!("Run connection command {:?}", &args);
        let mut command = Command::new(args.remove(0));
        command.args(args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to launch rsync subprocess {:?}", command))?;

        let r = Box::new(child.stdout.take().expect("Child has no stdout"));
        let w = Box::new(child.stdin.take().expect("Child has no stdin"));

        Connection::handshake(r, w, child, self.options.clone())
    }
}

#[derive(Debug)]
pub struct ParseAddressError {}

/// Builds an Client by matching the URL and SFTP-like formats used by
/// rsync.
impl FromStr for Client {
    type Err = ParseAddressError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        lazy_static! {
            static ref SFTP_RE: Regex = Regex::new(
                r"^(?x)
                    ((?P<user>[^@:]+)@)?
                    (?P<host>[^:@]+):
                    (?P<colon>:)?  # maybe a second colon, to indicate --daemon
                    (?P<path>.*)   # path; may be absolute or relative
                    $",
            )
            .unwrap();
            static ref URL_RE: Regex = Regex::new(
                r"^(?x)
                    rsync://
                    ((?P<user>[^@:]+)@)?
                    (?P<host>[^:/]+)
                    (:(?P<port>\d+))?
                    /
                    (?P<path>.*)
                    $",
            )
            .unwrap();
        }
        if let Some(caps) = URL_RE.captures(s) {
            Ok(Client {
                daemon: Some(Daemon {
                    host: caps["host"].into(),
                    user: caps.name("user").map(|m| m.as_str().to_string()),
                    port: caps.name("port").map(|p| p.as_str().parse().unwrap()),
                }),
                path: caps["path"].into(),
                ssh: None,
                options: Options::default(),
            })
        } else if let Some(caps) = SFTP_RE.captures(s) {
            if caps.name("colon").is_some() {
                Ok(Client {
                    path: caps["path"].into(),
                    daemon: Some(Daemon {
                        user: caps.name("user").map(|m| m.as_str().to_string()),
                        host: caps["host"].into(),
                        port: None,
                    }),
                    ssh: None,
                    options: Options::default(),
                })
            } else {
                Ok(Client {
                    path: caps["path"].into(),
                    ssh: Some(Ssh {
                        user: caps.name("user").map(|m| m.as_str().to_string()),
                        host: caps["host"].into(),
                    }),
                    daemon: None,
                    options: Options::default(),
                })
            }
        } else {
            // Assume it's just a path.
            Ok(Client {
                path: s.into(),
                ssh: None,
                daemon: None,
                options: Options::default(),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_sftp_style_without_user() {
        let address = Client::from_str("bilbo:/home/www").unwrap();
        assert_eq!(
            address,
            Client {
                ssh: Some(Ssh {
                    user: None,
                    host: "bilbo".into(),
                }),
                path: "/home/www".into(),
                daemon: None,
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_sftp_style_with_user() {
        let address = Client::from_str("mbp@bilbo:/home/www").unwrap();
        assert_eq!(
            address,
            Client {
                ssh: Some(Ssh {
                    user: Some("mbp".to_string()),
                    host: "bilbo".to_string(),
                }),
                path: "/home/www".into(),
                daemon: None,
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_daemon_simple() {
        let address = Client::from_str("rsync.samba.org::foo").unwrap();
        assert_eq!(
            address,
            Client {
                path: "foo".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: None,
                    port: None,
                }),
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_daemon_with_user() {
        let address = Client::from_str("rsync@rsync.samba.org::meat/bread/wine").unwrap();
        assert_eq!(
            address,
            Client {
                path: "meat/bread/wine".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: Some("rsync".into()),
                    port: None,
                }),
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_rsync_url() {
        let address = Client::from_str("rsync://rsync.samba.org/foo").unwrap();
        assert_eq!(
            address,
            Client {
                path: "foo".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: None,
                    port: None,
                }),
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_rsync_url_with_username() {
        let address = Client::from_str("rsync://anon@rsync.samba.org/foo").unwrap();
        assert_eq!(
            address,
            Client {
                path: "foo".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: Some("anon".into()),
                    port: None,
                }),
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_rsync_url_with_username_and_port() {
        let address =
            Client::from_str("rsync://anon@rsync.samba.org:8370/alpha/beta/gamma").unwrap();
        assert_eq!(
            address,
            Client {
                path: "alpha/beta/gamma".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: Some("anon".into()),
                    port: Some(8370),
                }),
                options: Options::default(),
            }
        );
    }

    #[test]
    fn parse_simple_path() {
        let address = Client::from_str("/usr/local/foo").unwrap();
        assert_eq!(
            address,
            Client {
                path: "/usr/local/foo".into(),
                ssh: None,
                daemon: None,
                options: Options::default(),
            }
        );
    }

    #[test]
    fn build_local_args() {
        let args = Client::local("./src").set_recursive(true).build_args();
        assert_eq!(args, vec!["rsync", "--server", "--sender", "-r", "./src"],);
    }

    #[test]
    fn build_local_args_with_rsync_path() {
        let args = Client::local("testdir")
            .set_options(Options {
                rsync_command: Some(vec!["/opt/rsync/rsync-3.1415".to_owned()]),
                ..Options::default()
            })
            .build_args();
        assert_eq!(
            args,
            ["/opt/rsync/rsync-3.1415", "--server", "--sender", "testdir"],
        );
    }

    #[test]
    fn build_local_args_verbose() {
        let mut address = Client::local("./src");
        address.set_verbose(3);
        let args = address.build_args();
        assert_eq!(args, ["rsync", "--server", "--sender", "-vvv", "./src"],);
    }

    #[test]
    fn build_ssh_args() {
        // Actually running SSH is a bit hard to test hermetically, but let's
        // at least check the command lines are plausible.

        let address = Client::ssh(None, "samba.org", "/home/mbp");
        let args = address.build_args();
        assert_eq!(
            args,
            [
                "ssh",
                "samba.org",
                "rsync",
                "--server",
                "--sender",
                "/home/mbp"
            ],
        );
    }

    #[test]
    fn build_ssh_args_with_user() {
        let mut address = Client::ssh(Some("mbp"), "samba.org", "/home/mbp");
        {
            let mut options = address.mut_options();
            options.recursive = true;
            options.list_only = true;
        }
        let args = address.build_args();
        assert_eq!(
            args,
            [
                "ssh",
                "-l",
                "mbp",
                "samba.org",
                "rsync",
                "--server",
                "--sender",
                "--list-only",
                "-r",
                "/home/mbp"
            ],
        );
    }

    #[test]
    fn build_ssh_args_with_ssh_command() {
        let ssh_args = ["/opt/openssh/ssh", "-A", "-DFoo=bar qux"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let args = Client::from_str("mbp@bilbo:/home/www")
            .unwrap()
            .set_options(Options {
                ssh_command: Some(ssh_args),
                ..Options::default()
            })
            .build_args();
        assert_eq!(
            args,
            [
                "/opt/openssh/ssh",
                "-A",
                "-DFoo=bar qux",
                "-l",
                "mbp",
                "bilbo",
                "rsync",
                "--server",
                "--sender",
                "/home/www",
            ]
        );
    }

    /// SSH with no path should say '.', typically to look in the home
    /// directory.
    #[test]
    fn build_ssh_args_for_default_directory() {
        let mut address: Client = "example-host:".parse().unwrap();
        address.mut_options().list_only = true;
        let args = address.build_args();
        assert_eq!(
            args,
            [
                "ssh",
                "example-host",
                "rsync",
                "--server",
                "--sender",
                "--list-only",
                "."
            ],
        );
    }

    /// Daemon mode is not implemented yet.
    #[test]
    #[should_panic]
    fn daemon_connection_unimplemented() {
        let address: Client = "rsync.example.com::example".parse().unwrap();
        let _ = address.connect();
    }
}
