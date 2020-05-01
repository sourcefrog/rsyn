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

//! Manipulate Unix file mode bits.
//!
//! Every filesystem entry (or inode) on Unix has a bit field of
//! [mode bits](https://en.wikipedia.org/wiki/Modes_(Unix))
//! that describe both the type of the file and its permissions.
//!
//! These are classically displayed in the left of `ls` output, and the permissions
//! can be changed with `chmod`.
//!
//! The encoding is fairly standard across unices, and occurs in some file
//! formats and network protocols that might be seen on non-Unix platforms.
//!
//! This library isn't Unix-specific and doesn't depend on the underlying OS to
//! interpret the bits.
//!
//! For example, this can be used with the return value from
//! `std::os::unix::fs::MetadataExt::mode()`.
//!
//! The names of the predicate functions match `std::fs::FileType` and
//! `std::os::unix::fs::FileTypeExt`.

/// Return just the bits representing the type of file.
fn type_bits(mode: u32) -> u32 {
    (mode >> 12) & 0o17
}

/// Returns true if this mode represents a regular file.
///
/// ```
/// assert_eq!(unix_mode::is_file(0o0041777), false);
/// assert_eq!(unix_mode::is_file(0o0100640), true);
/// ```
pub fn is_file(mode: u32) -> bool {
    type_bits(mode) == 0o010
}

/// Returns true if this mode represents a directory.
///
/// ```
/// assert_eq!(unix_mode::is_dir(0o0041777), true);
/// assert_eq!(unix_mode::is_dir(0o0100640), false);
/// ```
pub fn is_dir(mode: u32) -> bool {
    type_bits(mode) == 0o004
}

/// Returns true if this mode represents a symlink.
///
/// ```
/// assert_eq!(unix_mode::is_symlink(0o0040755), false);
/// assert_eq!(unix_mode::is_symlink(0o0120755), true);
/// ```
pub fn is_symlink(mode: u32) -> bool {
    type_bits(mode) == 0o012
}

/// Returns true if this mode represents a fifo, also known as a named pipe.
pub fn is_fifo(mode: u32) -> bool {
    type_bits(mode) == 0o001
}

/// Returns true if this mode represents a character device.
pub fn is_char_device(mode: u32) -> bool {
    type_bits(mode) == 0o002
}

/// Returns true if this mode represents a block device.
pub fn is_block_device(mode: u32) -> bool {
    type_bits(mode) == 0o006
}

/// Returns true if this mode represents a Unix-domain socket.
pub fn is_socket(mode: u32) -> bool {
    type_bits(mode) == 0o014
}

/// Convert Unix mode bits to a text string describing type and permissions,
/// as shown in `ls`.
///
/// Examples:
/// ```
/// assert_eq!(unix_mode::to_string(0o0040755), "drwxr-xr-x");
/// assert_eq!(unix_mode::to_string(0o0100640), "-rw-r-----");
///
/// // Classic "sticky" directory
/// assert_eq!(unix_mode::to_string(0o0041777), "drwxrwxrwt");
///
/// // Char and block devices
/// assert_eq!(unix_mode::to_string(0o0020600), "crw-------");
/// assert_eq!(unix_mode::to_string(0o0060600), "brw-------");
///
/// // Symlink
/// assert_eq!(unix_mode::to_string(0o0120777), "lrwxrwxrwx");
///
/// ```
pub fn to_string(mode: u32) -> String {
    // This is decoded "by hand" here so that it'll work
    // on non-Unix platforms.

    fn bitset(a: u32, b: u32) -> bool {
        a & b != 0
    }

    fn permch(mode: u32, b: u32, ch: char) -> char {
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
        _ => '?',     // unknown
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

#[cfg(test)]
mod test {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    fn file_mode(s: &str) -> u32 {
        std::fs::metadata(s).unwrap().mode()
    }

    /// Test predicates against files likely to already exist on a Unix system.
    #[cfg(unix)]
    #[test]
    fn stat_existing_files() {
        assert!(is_dir(file_mode("/")));
        assert!(!is_file(file_mode("/")));
        assert!(is_file(file_mode("/etc/passwd")));
        assert!(is_char_device(file_mode("/dev/null")));

        // I don't know how to reliably find a block device across OSes, and
        // we can't make one (without root.)

        // TODO: Make a fifo, socket, etc, and stat them.
    }
}
