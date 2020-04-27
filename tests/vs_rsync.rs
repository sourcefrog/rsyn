//! Test this library's compatibility by running original Tridge rsync.

use std::io;

use rsyn::Connection;

use lazy_static::lazy_static;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref LOGGER_DONE: () = install_test_logger();
}

#[test]
fn list_files_etc() -> io::Result<()> {
    install_test_logger();
    let _flist = Connection::local_subprocess("/etc")?.list_files()?;
    Ok(())
}

#[test]
fn list_files_dev() -> io::Result<()> {
    install_test_logger();
    let _flist = Connection::local_subprocess("/dev")?.list_files()?;
    Ok(())
}

fn install_test_logger() {
    // This'll fail if called twice; don't worry.
    let _ = fern::Dispatch::new()
        .format(rsyn::logging::format_log)
        .level(log::LevelFilter::Debug)
        .chain(fern::Output::call(|record| println!("{}", record.args())))
        .apply();
}