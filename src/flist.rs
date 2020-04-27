//! File lists.

use std::io;

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

pub struct FileEntry {
    /// The name received as a byte string.
    // TODO: Perhaps this should be an OSString, but it's not necessarily in the
    // *local* OS's format.
    pub name: Vec<u8>,
    pub file_len: i64,
    pub mode: i32,
    pub mtime: i32,
}

pub type FileList = Vec<FileEntry>;

pub fn read_file_list(r: &mut ReadVarint) -> io::Result<FileList> {
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
    // TODO: Sort by strcmp.
    Ok(v)
}
