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
    file: PathBuf,
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();
    rsyn::logging::default_logging();
    let file_list = Connection::local_subprocess(&opt.file)?.list_files()?;
    for entry in file_list {
        println!("{}", &entry)
    }
    debug!("that's all folks");
    Ok(())
}
