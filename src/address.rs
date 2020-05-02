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
use regex::Regex;

use crate::{Connection, FileList, Options, Result, ServerStatistics};

/// SSH command name, to start it as a subprocess.
const SSH_COMMAND: &str = "ssh";
/// rsync command name, to start it as a subprocess either locally or remotely.
const RSYNC_COMMAND: &str = "rsync";

/// The address of an rsync server, including
/// information about how to open the connection.
///
/// Addresses can be parsed from strings:
/// ```
/// use std::str::FromStr;
/// let address = rsyn::Address::from_str("rsync.example.com::module")
///     .expect("Parse failed");
/// ```
///
/// Or constructed:
/// ```
/// let address = rsyn::Address::local("./src");
/// let address = rsyn::Address::ssh(Some("user"), "host.example.com", "./src");
/// ```
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Address {
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

impl Address {
    /// Builds an Address that, when connected, starts an `rsync --server` subprocess
    /// on the local machine.
    ///
    /// This is primarily useful for testing.
    pub fn local<P: AsRef<Path>>(path: P) -> Address {
        Address {
            path: path.as_ref().as_os_str().into(),
            ssh: None,
            daemon: None,
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
            daemon: None,
        }
    }

    /// Builds the arguments to start a connection subcommand, including the
    /// command name.
    fn build_args(&self, options: &Options) -> Result<Vec<OsString>> {
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
        push_str("-vv");
        if options.list_only {
            push_str("--list-only")
        }
        if options.recursive {
            push_str("-r")
        }
        if self.path.is_empty() {
            push_str(".")
        } else {
            v.push(self.path.clone())
        }
        Ok(v)
    }

    /// List files from the remote server.
    ///
    /// This implicitly sets the `list_only` option.
    pub fn list_files(&self, mut options: Options) -> Result<(FileList, ServerStatistics)> {
        options.list_only = true;
        self.connect(options)
            .context("Failed to connect")?
            .list_files()
            .context("Failed to list files")
    }

    /// Opens a connection to this address.
    ///
    /// The `Address` can be opened any number of times, but each `Connection`
    /// can only do a single operation.
    pub fn connect(&self, options: Options) -> Result<Connection> {
        if self.daemon.is_some() {
            todo!("daemon mode is not implemented yet");
        }
        let mut args = self.build_args(&options)?;
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

#[derive(Debug)]
pub struct ParseAddressError {}

/// Builds an Address by matching the URL and SFTP-like formats used by
/// rsync.
impl FromStr for Address {
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
            Ok(Address {
                daemon: Some(Daemon {
                    host: caps["host"].into(),
                    user: caps.name("user").map(|m| m.as_str().to_string()),
                    port: caps.name("port").map(|p| p.as_str().parse().unwrap()),
                }),
                path: caps["path"].into(),
                ssh: None,
            })
        } else if let Some(caps) = SFTP_RE.captures(s) {
            if caps.name("colon").is_some() {
                Ok(Address {
                    path: caps["path"].into(),
                    daemon: Some(Daemon {
                        user: caps.name("user").map(|m| m.as_str().to_string()),
                        host: caps["host"].into(),
                        port: None,
                    }),
                    ssh: None,
                })
            } else {
                Ok(Address {
                    path: caps["path"].into(),
                    ssh: Some(Ssh {
                        user: caps.name("user").map(|m| m.as_str().to_string()),
                        host: caps["host"].into(),
                    }),
                    daemon: None,
                })
            }
        } else {
            // Assume it's just a path.
            Ok(Address {
                path: s.into(),
                ssh: None,
                daemon: None,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_sftp_style_without_user() {
        let address = Address::from_str("bilbo:/home/www").unwrap();
        assert_eq!(
            address,
            Address {
                ssh: Some(Ssh {
                    user: None,
                    host: "bilbo".into(),
                }),
                path: "/home/www".into(),
                daemon: None,
            }
        );
    }

    #[test]
    fn parse_sftp_style_with_user() {
        let address = Address::from_str("mbp@bilbo:/home/www").unwrap();
        assert_eq!(
            address,
            Address {
                ssh: Some(Ssh {
                    user: Some("mbp".to_string()),
                    host: "bilbo".to_string(),
                }),
                path: "/home/www".into(),
                daemon: None,
            }
        );
    }

    #[test]
    fn parse_daemon_simple() {
        let address = Address::from_str("rsync.samba.org::foo").unwrap();
        assert_eq!(
            address,
            Address {
                path: "foo".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: None,
                    port: None,
                }),
            }
        );
    }

    #[test]
    fn parse_daemon_with_user() {
        let address = Address::from_str("rsync@rsync.samba.org::meat/bread/wine").unwrap();
        assert_eq!(
            address,
            Address {
                path: "meat/bread/wine".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: Some("rsync".into()),
                    port: None,
                }),
            }
        );
    }

    #[test]
    fn parse_rsync_url() {
        let address = Address::from_str("rsync://rsync.samba.org/foo").unwrap();
        assert_eq!(
            address,
            Address {
                path: "foo".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: None,
                    port: None,
                }),
            }
        );
    }

    #[test]
    fn parse_rsync_url_with_username() {
        let address = Address::from_str("rsync://anon@rsync.samba.org/foo").unwrap();
        assert_eq!(
            address,
            Address {
                path: "foo".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: Some("anon".into()),
                    port: None,
                }),
            }
        );
    }

    #[test]
    fn parse_rsync_url_with_username_and_port() {
        let address =
            Address::from_str("rsync://anon@rsync.samba.org:8370/alpha/beta/gamma").unwrap();
        assert_eq!(
            address,
            Address {
                path: "alpha/beta/gamma".into(),
                ssh: None,
                daemon: Some(Daemon {
                    host: "rsync.samba.org".into(),
                    user: Some("anon".into()),
                    port: Some(8370),
                }),
            }
        );
    }

    #[test]
    fn parse_simple_path() {
        let address = Address::from_str("/usr/local/foo").unwrap();
        assert_eq!(
            address,
            Address {
                path: "/usr/local/foo".into(),
                ssh: None,
                daemon: None,
            }
        );
    }

    #[test]
    fn build_local_args() {
        let args = Address::local("./src")
            .build_args(&Options {
                recursive: true,
                ..Options::default()
            })
            .unwrap();
        assert_eq!(
            args,
            vec!["rsync", "--server", "--sender", "-vv", "-r", "./src"],
        );
    }

    #[test]
    fn build_ssh_args() {
        // Actually running SSH is a bit hard to test hermetically, but let's
        // at least check the command lines are plausible.

        let args = Address::ssh(None, "samba.org", "/home/mbp")
            .build_args(&Options::default())
            .unwrap();
        assert_eq!(
            args,
            vec![
                "ssh",
                "samba.org",
                "rsync",
                "--server",
                "--sender",
                "-vv",
                "/home/mbp"
            ],
        );
    }

    #[test]
    fn build_ssh_args_with_user() {
        let args = Address::ssh(Some("mbp"), "samba.org", "/home/mbp")
            .build_args(&Options {
                recursive: true,
                list_only: true,
            })
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
                "-vv",
                "--list-only",
                "-r",
                "/home/mbp"
            ],
        );
    }

    /// SSH with no path should say '.', typically to look in the home
    /// directory.
    #[test]
    fn build_ssh_args_for_default_directory() {
        let address: Address = "example-host:".parse().unwrap();
        let args = address
            .build_args(&Options {
                list_only: true,
                ..Options::default()
            })
            .unwrap();
        assert_eq!(
            args,
            vec![
                "ssh",
                "example-host",
                "rsync",
                "--server",
                "--sender",
                "-vv",
                "--list-only",
                "."
            ],
        );
    }

    /// Daemon mode is not implemented yet.
    #[test]
    #[should_panic]
    fn daemon_connection_unimplemented() {
        let address: Address = "rsync.example.com::example".parse().unwrap();
        let _ = address.connect(Options::default());
    }
}
