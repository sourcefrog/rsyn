/// Convert unix mode bits to a typical text string as shown in ls.
///
/// Examples:
/// ```
/// assert_eq!(unix_mode::to_string(0o0040755), "drwxr-xr-x");
/// assert_eq!(unix_mode::to_string(0o0100640), "-rw-r-----");
/// // Classic sticky directory
/// assert_eq!(unix_mode::to_string(0o0041777), "drwxrwxrwt");
/// // Char and block devices
/// assert_eq!(unix_mode::to_string(0o0020600), "crw-------");
/// assert_eq!(unix_mode::to_string(0o0060600), "brw-------");
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
