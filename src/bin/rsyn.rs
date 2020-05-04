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

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use structopt::StructOpt;

use rsyn::{Address, Options, Result};

#[derive(Debug, StructOpt)]
#[structopt()]
/// [pre-alpha] Wire-compatible rsync client in Rust.
///
/// With one PATH argument, lists the contents of that directory.
struct Opt {
    /// Directory to list.
    path: String,

    /// Turn on verbose debugging output.
    // TODO: Perhaps take an optarg controlling filtering per module?
    #[structopt(long)]
    debug: bool,

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
        }
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let log_level = if opt.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    fern::Dispatch::new()
        .format(rsyn::logging::format_log)
        .level(log_level)
        .chain(std::io::stderr())
        .chain(fern::log_file("rsyn.log").expect("failed to open log file"))
        .apply()
        .expect("Failed to configure logger");

    let address: Address = opt.path.parse().expect("Failed to parse path");
    let options = Options {
        list_only: true,
        ..opt.to_options()
    };
    let (file_list, _stats) = address.list_files(options)?;
    for entry in file_list {
        println!("{}", &entry)
    }
    debug!("that's all folks");
    Ok(())
}
