//! Library for wire-compatible rsync client in Rust.

use std::fmt;

mod connection;
mod flist;
mod mux;
mod varint;
mod statistics;

pub use connection::Connection;

pub fn format_log(out: fern::FormatCallback, args: &fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
        "[{}][{}] {}",
        record.target(),
        record.level().to_string().chars().next().unwrap(),
        args
    ))
}

pub fn default_logging() {
    fern::Dispatch::new()
        .format(format_log)
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("rsyn.log").expect("failed to open log file"))
        .apply()
        .expect("failed to configure logger")
}
