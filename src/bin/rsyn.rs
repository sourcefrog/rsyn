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

//! Command-line program for rsyn, an rsync client in Rust.

// use std::fmt;
use std::path::PathBuf;

// use anyhow::Context;
use structopt::StructOpt;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn, Level};

use rsyn::{Client, Options, Result};

#[derive(Debug, StructOpt)]
#[structopt()]
/// [pre-alpha] Wire-compatible rsync client in Rust.
///
/// With one PATH argument, lists the contents of that directory.
struct Opt {
    /// Directory to list.
    path: String,

    /// File to send log/debug messages.
    #[structopt(long, env = "RSYN_LOG_FILE")]
    log_file: Option<PathBuf>,

    /// Shell command to run to start rsync server.
    #[structopt(long, env = "RSYN_RSYNC_PATH")]
    rsync_path: Option<String>,

    /// Shell command to open a connection to a remote server (default is ssh).
    #[structopt(long, short = "e", env = "RSYN_RSH")]
    rsh: Option<String>,

    /// Recurse into directories.
    #[structopt(long, short = "r")]
    recursive: bool,

    /// List files, don't copy them.
    #[structopt(long)]
    list_only: bool,

    /// Be more verbose.
    #[structopt(short = "v", parse(from_occurrences))]
    verbose: u32,
}

impl Opt {
    /// Convert command-line options to protocol options.
    fn to_options(&self) -> Options {
        Options {
            recursive: self.recursive,
            list_only: self.list_only,
            verbose: self.verbose,
            rsync_command: self.rsync_path.as_ref().map(|p| {
                shell_words::split(&p).expect("Failed to split shell words from rsync_command")
            }),
            ssh_command: self.rsh.as_ref().map(|p| {
                shell_words::split(&p).expect("Failed to split shell words from ssh_command")
            }),
        }
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    configure_logging(&opt)?;

    let mut client = Client::from_str(&opt.path).expect("Failed to parse path");
    *client.mut_options() = opt.to_options();
    let (file_list, _stats) = client.list_files()?;
    for entry in file_list {
        println!("{}", &entry)
    }
    debug!("that's all folks");
    Ok(())
}

// Configure the logger: send everything to the log file (if there is one), and
// send info and above to the console.
fn configure_logging(opt: &Opt) -> Result<()> {
    let console_level = match opt.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(console_level)
        .with_writer(std::io::stderr)
        .init();

    // TODO: tracing currently can't support different levels per destination, and logging to a
    // file is confusing so also unsupported.

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rsync_path_option() {
        let opt = Opt::from_iter(&[
            "rsyn",
            "--rsync-path=rsync --wibble --wobble",
            "-vv",
            "/example",
        ]);
        assert_eq!(
            opt.rsync_path.as_deref().unwrap(),
            "rsync --wibble --wobble"
        );
        let options = opt.to_options();
        assert_eq!(
            options.rsync_command.unwrap(),
            ["rsync", "--wibble", "--wobble"]
        );
    }

    #[test]
    fn rsh_option() {
        let opt = Opt::from_iter(&["rsyn", "--rsh=ssh -OFoo -OBar=123 -v -A", "-vv", "/example"]);
        assert!(opt.rsync_path.is_none());
        let options = opt.to_options();
        assert!(options.rsync_command.is_none());
        assert_eq!(
            options.ssh_command.unwrap(),
            ["ssh", "-OFoo", "-OBar=123", "-v", "-A"]
        );
    }
}
