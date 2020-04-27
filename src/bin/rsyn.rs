use std::io;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rsyn::Connection;

fn main() -> io::Result<()> {
    rsyn::logging::default_logging();
    let file_list = Connection::local_subprocess("/etc")?.list_files()?;
    for entry in file_list {
        println!("{}", String::from_utf8_lossy(&entry.name));
    }
    debug!("that's all folks");
    Ok(())
}
