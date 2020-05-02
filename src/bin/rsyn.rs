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

use std::path::PathBuf;

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
    path: PathBuf,

    /// Turn on verbose debugging output.
    // TODO: Perhaps take an optarg controlling filtering per module?
    #[structopt(long)]
    debug: bool,
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

    // let address = Address::ssh(None, "localhost", opt.path.to_str().unwrap());
    let address = Address::local(&opt.path);

    let (file_list, _stats) = address.connect(Options::default())?.list_files()?;
    for entry in file_list {
        println!("{}", &entry)
    }
    debug!("that's all folks");
    Ok(())
}
