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

/// Return just the bits representing the type of file.
fn type_bits(mode: i32) -> i32 {
    (mode >> 12) & 0o17
}

/// Returns true if this mode represents a regular file.
///
/// ```
/// assert_eq!(unix_mode::is_file(0o0041777), false);
/// assert_eq!(unix_mode::is_file(0o0100640), true);
/// ```
pub fn is_file(mode: i32) -> bool {
    type_bits(mode) == 0o010
}

/// Returns true if this mode represents a directory.
///
/// ```
/// assert_eq!(unix_mode::is_dir(0o0041777), true);
/// assert_eq!(unix_mode::is_dir(0o0100640), false);
/// ```
pub fn is_dir(mode: i32) -> bool {
    type_bits(mode) == 0o004
}

/// Returns true if this mode represents a symlink.
///
/// ```
/// assert_eq!(unix_mode::is_symlink(0o0040755), false);
/// assert_eq!(unix_mode::is_symlink(0o0120755), true);
/// ```
pub fn is_symlink(mode: i32) -> bool {
    type_bits(mode) == 0o012
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
pub fn to_string(mode: i32) -> String {
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
