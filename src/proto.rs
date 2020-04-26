use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// TODO: Maybe instead of extensions, make an opaque wrapper, and never use raw
// read and write.

/// Extension trait to read typical rsync protocol components.
pub trait ReadProto {
    fn read_u8(&mut self) -> io::Result<u8>;
    fn read_byte_string(&mut self, len: usize) -> io::Result<Vec<u8>>;
    fn read_i32(&mut self) -> io::Result<i32>;
    fn read_i64(&mut self) -> io::Result<i64>;
    fn read_server_statistics(&mut self) -> io::Result<ServerStatistics>;
}

#[derive(Debug)]
pub struct ServerStatistics {
    pub total_bytes_read: i64,
    pub total_bytes_written: i64,
    pub total_file_size: i64,
    pub flist_build_time: i64,
    pub flist_xfer_time: i64,
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

    fn read_server_statistics(&mut self) -> io::Result<ServerStatistics> {
        // TODO: Perhaps this should be part of the Connection, or some place
        // that knows the protocol version.
        Ok(ServerStatistics {
            total_bytes_read: self.read_i64()?,
            total_bytes_written: self.read_i64()?,
            total_file_size: self.read_i64()?,
            // TODO: These last two are only set for protocol >=29.
            flist_build_time: self.read_i64()?,
            flist_xfer_time: self.read_i64()?,
        })
    }
}

/// Extension trait to write rsync low-level protocol components.
pub trait WriteProto {
    fn write_i32(&mut self, v: i32) -> io::Result<()>;
    fn write_u8(&mut self, v: u8) -> io::Result<()>;
}

impl WriteProto for dyn Write {
    fn write_i32(&mut self, v: i32) -> io::Result<()> {
        // debug!("send {:#x}", v);
        self.write_all(&v.to_le_bytes())
    }

    fn write_u8(&mut self, v: u8) -> io::Result<()> {
        // debug!("send {:#x}", v);
        self.write_all(&[v])
    }
}
