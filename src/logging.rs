//! Log helper functions.

use std::fmt;

/// Format a `log::Record`.
///
/// This is exposed just as a convenience for tests or other users. Any logger
/// configuration should work.
pub fn format_log(out: fern::FormatCallback, args: &fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
        "[{}][{}] {}",
        record.target(),
        record.level().to_string().chars().next().unwrap(),
        args
    ))
}
