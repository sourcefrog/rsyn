#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

mod connection;
mod flist;
mod mux;
mod proto;

use connection::Connection;

fn setup_logger() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("rsyn.log").expect("failed to open log file"))
        .apply()
        .expect("failed to configure logger")
}

fn main() {
    setup_logger();
    let mut conn = Connection::local_subprocess();
    conn.list_files();
    debug!("that's all folks");
}
