use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// Extension trait to read rsync variable-integer encodings and known-length
/// byte strings.
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
        // Smaller values are encoded as 4 bytes.
        let v = self.read_i32()?;
        if v < i32::MAX {
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
}

/// Write rsync low-level protocol variable integers.
pub struct WriteVarint {
    w: Box<dyn io::Write>,
}

impl WriteVarint {
    pub fn new(w: Box<dyn io::Write>) -> WriteVarint {
        WriteVarint{ w }
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
