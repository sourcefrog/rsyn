/// Library for wire-compatible rsync client in Rust.

mod connection;
mod flist;
mod mux;
mod varint;
mod statistics;

pub use connection::Connection;

pub fn default_logging() {
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
