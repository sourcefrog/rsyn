//! Test this library's compatibility by running original Tridge rsync.

use std::fs::File;

use anyhow::Result;
use tempdir::TempDir;

use rsyn::Connection;

/// List files from a newly-created temporary directory.
#[test]
fn list_files() {
    install_test_logger();

    let tmp = TempDir::new("rsyn_interop_list_files").unwrap();
    File::create(tmp.path().join("a")).unwrap();
    File::create(tmp.path().join("b")).unwrap();

    let flist = Connection::local_subprocess(tmp.path())
        .unwrap()
        .list_files()
        .unwrap();

    assert_eq!(flist.len(), 3);
    let names: Vec<String> = flist
        .iter()
        .map(|fe| fe.name_lossy_string().into_owned())
        .collect();
    // Names should already be sorted.
    assert_eq!(names[0], ".");
    assert_eq!(names[1], "a");
    assert_eq!(names[2], "b");
    // TODO: Check file types.
}

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
