use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// Extension trait to read typical rsync protocol components.
pub trait ReadProto {
    fn read_u8(&mut self) -> io::Result<u8>;
    fn read_byte_string(&mut self, len: usize) -> io::Result<Vec<u8>>;
    fn read_i32(&mut self) -> io::Result<i32>;
    fn read_i64(&mut self) -> io::Result<i64>;
}

impl ReadProto for dyn Read {
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut b = [0u8];
        self.read_exact(&mut b).and(Ok(b[0]))
    }

    fn read_byte_string(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf).and(Ok(buf))
    }

    fn read_i32(&mut self) -> io::Result<i32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_i64(&mut self) -> io::Result<i64> {
        // Smaller values are encoded as 4 bytes.
        let v = self.read_i32()?;
        if v < i32::MAX {
            Ok(v as i64)
        } else {
            let mut buf = [0; 8];
            self.read_exact(&mut buf)?;
            Ok(i64::from_le_bytes(buf))
        }
    }
}

/// Extension trait to write rsync low-level protocol components.
pub trait WriteProto {
    fn write_i32(&mut self, v: i32) -> io::Result<()>;
    fn write_u8(&mut self, v: u8) -> io::Result<()>;
}

impl WriteProto for dyn Write {
    fn write_i32(&mut self, v: i32) -> io::Result<()> {
        debug!("send {:#x}", v);
        self.write_all(&v.to_le_bytes())
    }

    fn write_u8(&mut self, v: u8) -> io::Result<()> {
        debug!("send {:#x}", v);
        self.write_all(&[v])
    }
}
