//! Read and write rsync's integer encoding scheme: u8, i32, i64, and byte strings.

use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub struct ReadVarint {
    r: Box<dyn Read>,
}

impl ReadVarint {
    pub fn new(r: Box<dyn Read>) -> ReadVarint {
        ReadVarint { r }
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        let mut b = [0u8];
        self.r.read_exact(&mut b).and(Ok(b[0]))
    }

    pub fn read_byte_string(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.r.read_exact(&mut buf).and(Ok(buf))
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        let mut buf = [0; 4];
        self.r.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn read_i64(&mut self) -> io::Result<i64> {
        let v = self.read_i32()?;
        if v != -1 {
            Ok(v as i64)
        } else {
            let mut buf = [0; 8];
            self.r.read_exact(&mut buf)?;
            Ok(i64::from_le_bytes(buf))
        }
    }

    /// Return the underlying stream, consuming this wrapper.
    pub fn take(self) -> Box<dyn Read> {
        self.r
    }

    // Destructively test that this is at the end of the input.
    #[allow(unused)]
    pub fn assert_is_at_end(mut self) {
        assert_eq!(
            self.read_u8().unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
}

/// Write rsync low-level protocol variable integers.
pub struct WriteVarint {
    w: Box<dyn io::Write>,
}

impl WriteVarint {
    pub fn new(w: Box<dyn io::Write>) -> WriteVarint {
        WriteVarint { w }
    }

    pub fn write_i32(&mut self, v: i32) -> io::Result<()> {
        // debug!("send {:#x}", v);
        self.w.write_all(&v.to_le_bytes())
    }

    #[allow(unused)]
    pub fn write_u8(&mut self, v: u8) -> io::Result<()> {
        // debug!("send {:#x}", v);
        self.w.write_all(&[v])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_rv(s: &'static [u8]) -> ReadVarint {
        ReadVarint::new(Box::new(s))
    }

    #[test]
    fn read_i64() {
        let mut rv = make_rv(&[0x10, 0, 0, 0]);
        assert_eq!(rv.read_i64().unwrap(), 0x10);

        let mut rv = make_rv(&[
            0xff, 0xff, 0xff, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        ]);
        assert_eq!(rv.read_i64().unwrap(), 0x7766554433221100);
        rv.assert_is_at_end();
    }
}
