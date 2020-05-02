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

/// Description of a single file (or directory or symlink etc).
///
/// The `Display` trait formats an entry like in `ls -l`, and like in rsync
/// directory listings.
#[derive(Debug, PartialEq, Eq)]
pub struct FileEntry {
    name: Vec<u8>,

    /// Length of the file, in bytes.
    pub file_len: u64,

    /// Unix mode, containing the file type and permissions.
    pub mode: u32,

    /// Modification time, in seconds since the Unix epoch.
    mtime: u32,
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
    pub fn name_lossy_string(&self) -> std::borrow::Cow<str> {
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

pub type FileList = Vec<FileEntry>;

/// Read a file list off the wire, and return it in sorted order.
pub(crate) fn read_file_list(r: &mut ReadVarint) -> Result<FileList> {
    // TODO: Support receipt of uid and gid with -o, -g.
    // TODO: Support devices, links, etc.

    let mut v: Vec<FileEntry> = Vec::new();
    loop {
        let status = r.read_u8()?;
        debug!("file list status {:#x}", status);
        if status == 0 {
            break;
        }

        // The name can be given in several ways:
        // * Fully specified with a byte length.
        // * Fully specified with an int length.
        // * Partially repeated, with a byte specifying how much is
        //   inherited.
        let inherit_name_bytes = if (status & STATUS_REPEAT_PARTIAL_NAME) != 0 {
            r.read_u8()? as usize
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
            let mut new_name = v.last().unwrap().name.clone();
            new_name.truncate(inherit_name_bytes);
            new_name.append(&mut name);
            name = new_name;
        }
        debug!("  filename: {:?}", String::from_utf8_lossy(&name));
        assert!(!name.is_empty());

        let file_len: u64 = r
            .read_i64()?
            .try_into()
            .context("Received negative file_len")?;
        debug!("  file_len: {}", file_len);

        let mtime = if status & STATUS_REPEAT_MTIME == 0 {
            r.read_i32()? as u32
        } else {
            v.last().unwrap().mtime
        };
        debug!("  mtime: {}", mtime);

        let mode = if status & STATUS_REPEAT_MODE == 0 {
            r.read_i32()? as u32
        } else {
            v.last().unwrap().mode
        };
        debug!("  mode: {:#o}", mode);

        v.push(FileEntry {
            name,
            file_len,
            mtime,
            mode,
        });
    }
    debug!("end of file list");
    v.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    // TODO: Sort by strcmp.
    Ok(v)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn file_entry_display_like_ls() {
        let entry = FileEntry {
            mode: 0o0040750,
            file_len: 420,
            mtime: 1588429517,
            name: b"rsyn".to_vec(),
        };
        assert_eq!(
            format!("{}", entry),
            "drwxr-x---         420 2020-05-02 07:25:17 rsyn"
        );
    }
}
