//! File lists and entries.

use std::fmt;
use std::io;

use chrono::{Local, TimeZone};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::varint::ReadVarint;

// const STATUS_TOP_LEVEL_DIR: u8 = 0x01;
const STATUS_REPEAT_MODE: u8 = 0x02;
// const STATUS_REPEAT_UID: u8 = 0x08;
// const STATUS_REPEAT_GID: u8 = 0x08;
const STATUS_REPEAT_PARTIAL_NAME: u8 = 0x20;
const STATUS_LONG_NAME: u8 = 0x40;
const STATUS_REPEAT_MTIME: u8 = 0x80;

/// Description of a single file (or directory or symlink etc).
pub struct FileEntry {
    name: Vec<u8>,

    /// Length of the file, in bytes.
    pub file_len: i64,

    /// Unix mode, containing the file type and permissions.
    pub mode: i32,

    /// Modification time, in seconds since the Unix epoch.
    pub mtime: i32,
}

impl FileEntry {
    pub fn mtime_timestamp(&self) -> chrono::DateTime<Local> {
        Local.timestamp(self.mtime as i64, 0)
    }

    /// Return the file name, as a byte string, in the (remote) OS's encoding.
    ///
    /// rsync doesn't constrain the encoding, so this will typically, but not
    /// necessarily be UTF-8.
    // TODO: Also offer it as an OSString?
    pub fn name_bytes(&self) -> &[u8] {
        &self.name
    }

    /// Return the name, with un-decodable bytes converted to Unicode
    /// replacement characters.
    ///
    /// For the common case of UTF-8 names, this is simply the name, but
    /// if the remote end uses a different encoding the name may be mangled.
    pub fn name_lossy_string(&self) -> std::borrow::Cow<str> {
        String::from_utf8_lossy(&self.name)
    }
}

/// Display this entry in a format like that of `ls` and like `rsync` uses in
/// listing directories:
///
/// ```text
/// lrwxr-xr-x          11 2020/02/28 07:33:44 etc
/// ```
impl fmt::Display for FileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08} {:11} {:19} {}",
            unix_mode::to_string(self.mode),
            self.file_len,
            self.mtime_timestamp().format("%Y-%m-%d %H:%M:%S"),
            self.name_lossy_string(),
        )
    }
}

pub type FileList = Vec<FileEntry>;

/// Read a file list off the wire, and return it in sorted order.
pub(crate) fn read_file_list(r: &mut ReadVarint) -> io::Result<FileList> {
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

        let file_len = r.read_i64()?;
        debug!("  file_len: {}", file_len);

        let mtime = if status & STATUS_REPEAT_MTIME == 0 {
            r.read_i32()?
        } else {
            v.last().unwrap().mtime
        };
        debug!("  mtime: {}", mtime);

        let mode = if status & STATUS_REPEAT_MODE == 0 {
            r.read_i32()?
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
