// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test this library's compatibility by running original Tridge rsync.

use std::fs::{create_dir, File};

use anyhow::Result;
use chrono::prelude::*;
use tempdir::TempDir;

use rsyn::{Address, Options};

/// List files from a newly-created temporary directory.
#[test]
fn list_files() {
    install_test_logger();

    let tmp = TempDir::new("rsyn_interop_list_files").unwrap();
    File::create(tmp.path().join("a")).unwrap();
    File::create(tmp.path().join("b")).unwrap();
    create_dir(tmp.path().join("subdir")).unwrap();
    File::create(tmp.path().join("subdir").join("galah")).unwrap();

    let (flist, stats) = Address::local(tmp.path())
        .list_files(Options {
            recursive: true,
            ..Options::default()
        })
        .unwrap();

    assert_eq!(flist.len(), 5);
    let names: Vec<String> = flist
        .iter()
        .map(|fe| fe.name_lossy_string().into_owned())
        .collect();
    // Names should already be sorted.
    assert_eq!(names[0], ".");
    assert_eq!(names[1], "a");
    assert_eq!(names[2], "b");
    assert_eq!(names[3], "subdir");
    assert_eq!(names[4], "subdir/galah");

    // Check file types.
    assert!(flist[0].is_dir());
    assert!(!flist[0].is_file());
    assert!(
        flist[1].is_file(),
        "expected {:?} would be a file",
        &flist[1]
    );
    assert!(flist[2].is_file());
    assert!(flist[3].is_dir());
    assert!(flist[4].is_file());

    // Check mtimes. We don't control them precisely, but they should be close
    // to the current time. (Probably within a couple of seconds, but allow
    // some slack for debugging, thrashing machines, etc.)
    let now = Local::now();
    assert!((now - flist[0].mtime()).num_minutes() < 5);
    assert!((now - flist[1].mtime()).num_minutes() < 5);

    // All the files are empty.
    assert_eq!(stats.total_file_size, 0);
}

/// Only on Unix, check we can list a directory containing a symlink, and see
/// the symlink.
#[cfg(unix)]
#[test]
fn list_symlink() -> rsyn::Result<()> {
    install_test_logger();

    let tmp = TempDir::new("rsyn_interop_list_symlink")?;
    std::os::unix::fs::symlink("dangling link", tmp.path().join("a link"))?;

    let options = Options {
        list_only: true,
        recursive: false,
    };
    let (flist, _stats) = Address::local(tmp.path()).list_files(options)?;

    assert_eq!(flist.len(), 2);
    assert_eq!(flist[0].name_lossy_string(), ".");
    assert_eq!(flist[1].name_lossy_string(), "a link");

    assert!(!flist[0].is_symlink());
    assert!(flist[1].is_symlink());

    Ok(())
}

/// Only on Unix: list `/etc`, a good natural source of files with different
/// permissions, including some probably not readable to the non-root
/// user running this test.
#[cfg(unix)]
#[test]
fn list_files_etc() -> Result<()> {
    install_test_logger();
    let (_flist, _stats) = Address::local("/etc").list_files(Options {
        recursive: true,
        list_only: true,
    })?;
    Ok(())
}

/// Only on Unix: list `/dev`, a good source of devices and unusual files.
#[cfg(unix)]
#[test]
fn list_files_dev() -> Result<()> {
    install_test_logger();
    let (_flist, _stats) = Address::local("/dev").list_files(Options {
        recursive: true,
        list_only: true,
    })?;
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
