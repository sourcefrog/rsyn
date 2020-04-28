//! Test this library's compatibility by running original Tridge rsync.

use anyhow::Result;

use rsyn::Connection;

#[cfg(unix)]
#[test]
/// Only on Unix: list `/etc`, a good natural source of files with different
/// permissions, including some probably not readable to the non-root
/// user running this test.
fn list_files_etc() -> Result<()> {
    install_test_logger();
    let _flist = Connection::local_subprocess("/etc")?.list_files()?;
    Ok(())
}

#[cfg(unix)]
#[test]
/// Only on Unix: list `/dev`, a good source of devices and unusual files.
fn list_files_dev() -> Result<()> {
    install_test_logger();
    let _flist = Connection::local_subprocess("/dev")?.list_files()?;
    Ok(())
}

fn install_test_logger() {
    // The global logger can only be installed once per process, but this'll be called for
    // many tests within the same process. They all try to install the same thing, so don't
    // worry if it fails.
    let _ = fern::Dispatch::new()
        .format(rsyn::logging::format_log)
        .level(log::LevelFilter::Debug)
        .chain(fern::Output::call(|record| println!("{}", record.args())))
        .apply();
}
