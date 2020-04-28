//! Log helper functions.

use std::fmt;

pub fn format_log(out: fern::FormatCallback, args: &fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
        "[{}][{}] {}",
        record.target(),
        record.level().to_string().chars().next().unwrap(),
        args
    ))
}
