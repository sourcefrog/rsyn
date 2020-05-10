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

use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt;

use anyhow::Context;
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
#[derive(Debug, PartialEq, Eq)]
pub struct FileEntry {
    // Corresponds to rsync |file_struct|.
    /// Name of this file, as a byte string.
    name: Vec<u8>,

    /// Index in `name` of the last `'/'`.
    last_slash: Option<usize>,

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
    // TODO: Also offer it as an OSString?
    pub fn name_bytes(&self) -> &[u8] {
        &self.name
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

    /// Return the directory name, defined as the substring up to the last `'/'`
    /// if any. In the root directory, this is an empty slice.
    pub fn dirname(&self) -> &[u8] {
        &self.name[..self.last_slash.unwrap_or_default()]
    }

    /// Return the base file name, after the last slash (if any).
    pub fn basename(&self) -> &[u8] {
        match self.last_slash {
            None => &self.name,
            Some(p) => &self.name[(p + 1)..],
        }
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

/// Read a file list off the wire, and return it in the order it was received.
pub(crate) fn read_file_list(r: &mut ReadVarint) -> Result<FileList> {
    // Corresponds to rsync |receive_file_entry|.
    // TODO: Support receipt of uid and gid with -o, -g.
    // TODO: Support devices, links, etc.

    let mut v: Vec<FileEntry> = Vec::new();
    while let Some(entry) = receive_file_entry(r, v.last())? {
        v.push(entry)
    }
    debug!("End of file list");
    Ok(v)
}

fn receive_file_entry(
    r: &mut ReadVarint,
    previous: Option<&FileEntry>,
) -> Result<Option<FileEntry>> {
    let status = r
        .read_u8()
        .context("Failed to read file entry status byte")?;
    trace!("File list status {:#x}", status);
    if status == 0 {
        return Ok(None);
    }

    let inherit_name_bytes = if (status & STATUS_REPEAT_PARTIAL_NAME) != 0 {
        r.read_u8().context("Failed to read inherited name bytes")? as usize
    } else {
        0
    };

    let name_len = if status & STATUS_LONG_NAME != 0 {
        r.read_i32()? as usize
    } else {
        r.read_u8()? as usize
    };
    let mut name = r.read_byte_string(name_len)?;
    if inherit_name_bytes > 0 {
        let mut new_name = previous.unwrap().name.clone();
        new_name.truncate(inherit_name_bytes);
        new_name.append(&mut name);
        name = new_name;
    }
    trace!("  filename: {:?}", String::from_utf8_lossy(&name));
    assert!(!name.is_empty());
    let last_slash = name.iter().rposition(|c| *c == b'/');

    let file_len: u64 = r
        .read_i64()?
        .try_into()
        .context("Received negative file_len")?;
    trace!("  file_len: {}", file_len);

    let mtime = if status & STATUS_REPEAT_MTIME == 0 {
        r.read_i32()? as u32
    } else {
        previous.unwrap().mtime
    };
    trace!("  mtime: {}", mtime);

    let mode = if status & STATUS_REPEAT_MODE == 0 {
        r.read_i32()? as u32
    } else {
        previous.unwrap().mode
    };
    trace!("  mode: {:#o}", mode);

    // TODO: If the relevant options are set, read uid, gid, device, link target.

    Ok(Some(FileEntry {
        name,
        last_slash,
        file_len,
        mtime,
        mode,
        link_target: None,
    }))
}

/// Compare two entry names, in the protocol 27 sort.
fn file_compare_27(a: &FileEntry, b: &FileEntry) -> Ordering {
    // Corresponds to |file_compare|.
    let a_base = a.basename();
    let b_base = b.basename();
    let a_dir = a.dirname();
    let b_dir = b.dirname();
    if a_base.is_empty() && b_base.is_empty() {
        Ordering::Equal
    } else if a_base.is_empty() {
        Ordering::Greater
    } else if b_base.is_empty() {
        Ordering::Less
    } else if a_dir == b_dir {
        a_base.cmp(&b_base)
    } else {
        a.name.cmp(&b.name)
    }
}

pub(crate) fn sort(file_list: &mut [FileEntry]) {
    // Compare to rsync `file_compare`.
    // TODO: Clean the list of duplicates, like in rsync `clean_flist`.
    file_list.sort_unstable_by(file_compare_27);
    debug!("File list sort done");
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
            last_slash: None,
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

    // TODO: Test reading and decoding from a byte string, including finding the
    // directory separator.

    // TODO: Test sorting.
}
