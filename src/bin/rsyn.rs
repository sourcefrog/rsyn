//! Command-line program for rsyn, an rsync client in Rust.

use std::io;
use std::path::PathBuf;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use structopt::StructOpt;

use rsyn::Connection;

#[derive(Debug, StructOpt)]
#[structopt()]
/// [pre-alpha] Wire-compatible rsync in Rust
///
/// At present this program can only recursively list the contents of a local
/// directory, but it does this by launching rsync and talking its network
/// protocol.
struct Opt {
    /// Directory to list.
    path: PathBuf,

    /// Turn on verbose debugging output.
    // TODO: Perhaps take an optarg controlling filtering per module?
    #[structopt(long)]
    debug: bool,
}

fn main() -> io::Result<()> {
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
        .expect("failed to configure logger");

    let file_list = Connection::local_subprocess(&opt.path)?.list_files()?;
    for entry in file_list {
        println!("{}", &entry)
    }
    debug!("that's all folks");
    Ok(())
}
