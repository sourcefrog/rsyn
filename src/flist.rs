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

//! File lists and entries.

use std::convert::TryInto;
use std::fmt;

use anyhow::{bail, Context};
use chrono::{Local, TimeZone};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::varint::ReadVarint;
use crate::Result;

// const STATUS_TOP_LEVEL_DIR: u8 = 0x01;
const STATUS_REPEAT_MODE: u8 = 0x02;
// const STATUS_REPEAT_UID: u8 = 0x08;
// const STATUS_REPEAT_GID: u8 = 0x08;
const STATUS_REPEAT_PARTIAL_NAME: u8 = 0x20;
const STATUS_LONG_NAME: u8 = 0x40;
const STATUS_REPEAT_MTIME: u8 = 0x80;

type ByteString = Vec<u8>;

/// Description of a single file (or directory or symlink etc).
///
/// The `Display` trait formats an entry like in `ls -l`, and like in rsync
/// directory listings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    // Corresponds to rsync |file_struct|.
    /// Name of this file, as a byte string.
    name: Vec<u8>,

    /// Length of the file, in bytes.
    pub file_len: u64,

    /// Unix mode, containing the file type and permissions.
    pub mode: u32,

    /// Modification time, in seconds since the Unix epoch.
    mtime: u32,

    /// If this is a symlink, the target.
    link_target: Option<ByteString>,
    // TODO: Other file_struct fields.
    // TODO: Work out what |basedir| is and maybe include that.
}

impl FileEntry {
    /// Returns the file name, as a byte string, in the (remote) OS's encoding.
    ///
    /// rsync doesn't constrain the encoding, so this will typically, but not
    /// necessarily be UTF-8.
    pub fn name_bytes(&self) -> &[u8] {
        &self.name
    }

    /// Returns the name as a string if possible.
    pub fn name_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.name)
            .with_context(|| format!("Failed to decode name {:?}", self.name_lossy_string()))
            .into()
    }

    /// Returns the file name, with un-decodable bytes converted to Unicode
    /// replacement characters.
    ///
    /// For the common case of UTF-8 names, this is simply the name, but
    /// if the remote end uses a different encoding the name may be mangled.
    ///
    /// This is suitable for printing, but might not be suitable for use as a
    /// destination file name.
    pub fn name_lossy_string(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.name)
    }

    /// Returns true if this entry describes a plain file.
    pub fn is_file(&self) -> bool {
        unix_mode::is_file(self.mode)
    }

    /// Returns true if this entry describes a directory.
    pub fn is_dir(&self) -> bool {
        unix_mode::is_dir(self.mode)
    }

    /// Returns true if this entry describes a symlink.
    pub fn is_symlink(&self) -> bool {
        unix_mode::is_symlink(self.mode)
    }

    /// Returns the modification time, in seconds since the Unix epoch.
    pub fn unix_mtime(&self) -> u32 {
        self.mtime
    }

    /// Returns the modification time as a chrono::DateTime associated to the
    /// local timezone.
    pub fn mtime(&self) -> chrono::DateTime<Local> {
        Local.timestamp(self.mtime as i64, 0)
    }
}

/// Display this entry in a format like that of `ls`, and like `rsync` uses in
/// listing directories:
///
/// ```text
/// drwxr-x---         420 2020-05-02 07:25:17 rsyn
/// ```
///
/// The modification time is shown in the local timezone.
impl fmt::Display for FileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08} {:11} {:19} {}",
            unix_mode::to_string(self.mode),
            self.file_len,
            self.mtime().format("%Y-%m-%d %H:%M:%S"),
            self.name_lossy_string(),
        )
    }
}

/// A list of files returned from a server.
pub type FileList = Vec<FileEntry>;

/// Reads a file list, and then cleans and sorts it.
pub(crate) fn read_file_list(rv: &mut ReadVarint) -> Result<FileList> {
    // Corresponds to rsync |receive_file_entry|.
    // TODO: Support receipt of uid and gid with -o, -g.
    // TODO: Support devices, links, etc.
    // TODO: Sort order changes in different protocol versions.

    let mut file_list = Vec::new();
    while let Some(entry) = receive_file_entry(rv, file_list.last())? {
        file_list.push(entry)
    }
    debug!("End of file list");
    sort_and_dedupe(&mut file_list);
    Ok(file_list)
}

fn receive_file_entry(
    rv: &mut ReadVarint,
    previous: Option<&FileEntry>,
) -> Result<Option<FileEntry>> {
    let status = rv
        .read_u8()
        .context("Failed to read file entry status byte")?;
    trace!("File list status {:#x}", status);
    if status == 0 {
        return Ok(None);
    }

    let inherit_name_bytes = if (status & STATUS_REPEAT_PARTIAL_NAME) != 0 {
        rv.read_u8()
            .context("Failed to read inherited name bytes")? as usize
    } else {
        0
    };

    let name_len = if status & STATUS_LONG_NAME != 0 {
        rv.read_i32()? as usize
    } else {
        rv.read_u8()? as usize
    };
    let mut name = rv.read_byte_string(name_len)?;
    if inherit_name_bytes > 0 {
        let mut new_name = previous.unwrap().name.clone();
        new_name.truncate(inherit_name_bytes);
        new_name.append(&mut name);
        name = new_name;
    }
    trace!("  filename: {:?}", String::from_utf8_lossy(&name));
    assert!(!name.is_empty());
    validate_name(&name)?;

    let file_len: u64 = rv
        .read_i64()?
        .try_into()
        .context("Received negative file_len")?;
    trace!("  file_len: {}", file_len);

    let mtime = if status & STATUS_REPEAT_MTIME == 0 {
        rv.read_i32()? as u32
    } else {
        previous.unwrap().mtime
    };
    trace!("  mtime: {}", mtime);

    let mode = if status & STATUS_REPEAT_MODE == 0 {
        rv.read_i32()? as u32
    } else {
        previous.unwrap().mode
    };
    trace!("  mode: {:#o}", mode);

    // TODO: If the relevant options are set, read uid, gid, device, link target.

    Ok(Some(FileEntry {
        name,
        file_len,
        mtime,
        mode,
        link_target: None,
    }))
}

/// Check that this name is safe to handle, and doesn't seem to include an escape from the
/// directory.
///
/// The resulting path should only ever be used as relative to a destination directory.
///
fn validate_name(name: &[u8]) -> Result<()> {
    // Compare to rsync |clean_fname| and |sanitize_path|, although this does not
    // yet have the behavior of mapping into a pseudo-chroot directory, and it
    // only treats bad names as errors.
    //
    // TODO: Also look for special device files on Windows?
    let printable = || String::from_utf8_lossy(name);
    if name.is_empty() {
        bail!("Invalid name: empty");
    }
    if name[0] == b'/' {
        bail!("Invalid name: absolute: {:?}", printable());
    }
    for part in name.split(|b| *b == b'/') {
        if part.is_empty() || part == b".." {
            bail!(
                "Unsafe file path {:?}: this is either mischief by the sender or a bug",
                printable()
            );
        }
    }
    Ok(())
}

fn sort_and_dedupe(file_list: &mut Vec<FileEntry>) {
    // Compare to rsync `file_compare`.

    // In the rsync protocol the receiver gets a list of files from the server in
    // arbitrary order, and then is required to sort them into the same order
    // as the server, so they can use the same index numbers to refer to identify
    // files. (It's a bit strange.)
    //
    // The ordering varies per protocol version but in protocol 27 it's essentially
    // strcmp. (The rsync code is a bit complicated by storing the names split
    // into directory and filename.)
    file_list.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    debug!("File list sort done");
    let len_before = file_list.len();
    file_list.dedup_by(|a, b| a.name == b.name);
    let removed = len_before - file_list.len();
    if removed > 0 {
        debug!("{} duplicate file list entries removed", removed)
    }
    for (i, entry) in file_list.iter().enumerate() {
        debug!("[{:8}] {:?}", i, entry.name_lossy_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use regex::Regex;

    #[test]
    fn file_entry_display_like_ls() {
        let entry = FileEntry {
            mode: 0o0040750,
            file_len: 420,
            mtime: 1588429517,
            name: b"rsyn".to_vec(),
            link_target: None,
        };
        // The mtime is in the local timezone, and we need the tests to pass
        // regardless of timezone. Rust Chrono doesn't seem to provide a way
        // to override it for testing. Let's just assert that the pattern is
        // plausible.
        //
        // This does assume there are no timezones with a less-than-whole minute
        // offset. (There are places like South Australia with a fractional-hour offset.
        let entry_display = format!("{}", entry);
        assert!(
            Regex::new(r"drwxr-x---         420 2020-05-0[123] \d\d:\d\d:17 rsyn")
                .unwrap()
                .is_match(&entry_display),
            "{:?} doesn't match expected format",
            entry_display
        );
    }

    // TODO: Test reading and decoding from an varint stream.

    /// Examples from verbose output of rsync 2.6.1.
    #[test]
    fn ordering_examples() {
        const EXAMPLE: &[&[u8]] = &[
            b"./",
            b".git/",
            b".git/HEAD",
            b".github/",
            b".github/workflows/",
            b".github/workflows/rust.yml",
            b".gitignore",
            b"CONTRIBUTING.md",
            b"src/",
            b"src/lib.rs",
        ];
        let clean: Vec<FileEntry> = EXAMPLE
            .iter()
            .map(|name| FileEntry {
                mode: 0o0040750,
                file_len: 420,
                mtime: 1588429517,
                name: name.to_vec(),
                link_target: None,
            })
            .collect();
        let mut messy = clean.clone();
        messy.reverse();
        messy.extend_from_slice(clean.as_slice());
        sort_and_dedupe(&mut messy);
        assert_eq!(&messy, &clean);
    }

    #[test]
    fn validate_name() {
        use super::validate_name;
        assert!(validate_name(b".").is_ok());
        assert!(validate_name(b"./ok").is_ok());
        assert!(validate_name(b"easy").is_ok());
        assert!(validate_name(b"../../naughty").is_err());
        assert!(validate_name(b"still/not/../ok").is_err());
    }
}
