use std::io;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rsyn::Connection;

fn main() -> io::Result<()> {
    rsyn::logging::default_logging();
    let conn = Connection::local_subprocess("/etc")?;
    conn.list_files()?;
    debug!("that's all folks");
    Ok(())
}
