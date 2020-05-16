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

//! Read and write rsync's integer encoding scheme: u8, i32, i64, and byte strings.

use std::io;
use std::io::prelude::*;

use anyhow::{Context, Error, Result};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// Read rsync data types from a wrapped stream.
pub struct ReadVarint {
    r: Box<dyn Read + Send>,
}

impl ReadVarint {
    pub fn new(r: Box<dyn Read + Send>) -> ReadVarint {
        ReadVarint { r }
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        let mut b = [0u8];
        self.r.read_exact(&mut b).and(Ok(b[0]))
    }

    /// Read a known-length byte string into a newly allocated buffer.
    ///
    /// Always returns the exact size, or an error.
    pub fn read_byte_string(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.r.read_exact(&mut buf).and(Ok(buf))
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        let mut buf = [0; 4];
        self.r.read_exact(&mut buf)?;
        let v = i32::from_le_bytes(buf);
        trace!("Read {:#x}i32", v);
        Ok(v)
    }

    pub fn read_i64(&mut self) -> io::Result<i64> {
        let v = self.read_i32()?;
        let v = if v != -1 {
            v as i64
        } else {
            let mut buf = [0; 8];
            self.r.read_exact(&mut buf)?;
            i64::from_le_bytes(buf)
        };
        trace!("Read {:#x}i64", v);
        Ok(v)
    }

    /// Return the underlying stream, consuming this wrapper.
    pub fn take(self) -> Box<dyn Read + Send> {
        self.r
    }

    /// Destructively test that this is at the end of the input.
    ///
    /// Returns an error either on an IO error, or if there's any remaining
    /// data. The remaining data will be unreachable (and corrupt.) Returns Ok(())
    /// on a clean end.
    #[allow(unused)]
    pub fn check_for_eof(mut self) -> Result<()> {
        match self.read_u8().and(Ok(())) {
            Ok(_) => Err(Error::msg("Data found when we expected end of stream")),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(()),
            e => e.context("Error looking for end of stream"),
        }
    }
}

/// Write rsync low-level protocol variable integers.
pub struct WriteVarint {
    w: Box<dyn io::Write + Send>,
}

impl WriteVarint {
    pub fn new(w: Box<dyn io::Write + Send>) -> WriteVarint {
        WriteVarint { w }
    }

    pub fn write_i32(&mut self, v: i32) -> io::Result<()> {
        trace!("Send {:#x}i32", v);
        self.w.write_all(&v.to_le_bytes())
    }

    #[allow(unused)]
    pub fn write_u8(&mut self, v: u8) -> io::Result<()> {
        trace!("Send {:#x}u8", v);
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
        rv.check_for_eof().unwrap();
    }
}
