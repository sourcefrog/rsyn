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

pub struct FileEntry {
    /// The name received as a byte string.
    // TODO: Perhaps this should be an OSString, but it's not necessarily in the
    // *local* OS's format.
    pub name: Vec<u8>,
    pub file_len: i64,
    pub mode: i32,
    pub mtime: i32,
}

impl FileEntry {
    pub fn mtime_timestamp(&self) -> chrono::DateTime<Local> {
        Local.timestamp(self.mtime as i64, 0)
    }
}

// lrwxr-xr-x          11 2020/02/28 07:33:44 etc
impl fmt::Display for FileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08} {:11} {:19} {}",
            unix_mode_to_string(self.mode),
            self.file_len,
            self.mtime_timestamp().format("%Y-%m-%d %H:%M:%S"),
            String::from_utf8_lossy(&self.name)
        )
    }
}

/// Convert unix mode bits to a typical text string.
fn unix_mode_to_string(mode: i32) -> String {
    // This is decoded "by hand" here so that it'll work
    // on non-Unix platforms.

    fn bitset(a: i32, b: i32) -> bool {
        a & b != 0
    }

    fn permch(mode: i32, b: i32, ch: char) -> char {
        if bitset(mode, b) {
            ch
        } else {
            '-'
        }
    }

    let mut s = String::with_capacity(10);
    s.push(match (mode >> 12) & 0o17 {
        0o001 => 'p', // pipe/fifo
        0o002 => 'c', // character dev
        0o004 => 'd', // directory
        0o006 => 'b', // block dev
        0o010 => '-', // regular file
        0o012 => 'l', // link
        0o014 => 's', // socket
        0o016 => 'w', // whiteout
        _ => panic!("incomprehensible mode {:#o}", mode),
    });
    let setuid = bitset(mode, 0o4000);
    let setgid = bitset(mode, 0o2000);
    let sticky = bitset(mode, 0o1000);
    s.push(permch(mode, 0o400, 'r'));
    s.push(permch(mode, 0o200, 'w'));
    let usrx = bitset(mode, 0o100);
    if setuid && usrx {
        s.push('s')
    } else if setuid && !usrx {
        s.push('S')
    } else if usrx {
        s.push('x')
    } else {
        s.push('-')
    }
    // group
    s.push(permch(mode, 0o40, 'r'));
    s.push(permch(mode, 0o20, 'w'));
    let grpx = bitset(mode, 0o10);
    if setgid && grpx {
        s.push('s')
    } else if setgid && !grpx {
        s.push('S')
    } else if grpx {
        s.push('x')
    } else {
        s.push('-')
    }
    // other
    s.push(permch(mode, 0o4, 'r'));
    s.push(permch(mode, 0o2, 'w'));
    let otherx = bitset(mode, 0o1);
    if sticky && otherx {
        s.push('t')
    } else if sticky && !otherx {
        s.push('T')
    } else if otherx {
        s.push('x')
    } else {
        s.push('-')
    }
    s
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
