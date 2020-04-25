#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

mod connection;
mod parser;

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
    let _conn = Connection::local_subprocess();
    debug!("that's all folks");
}
