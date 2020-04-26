use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

const TAG_DATA: u8 = 7;
const TAG_FATAL: u8 = 1;

pub struct DemuxRead {
    r: Box<dyn Read>,
    /// Amount of data from previous packet remaining to read out
    remains: usize,
}

impl Read for DemuxRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remains == 0 {
            self.remains = self.read_header_consume_messages()?;
        }
        let max_len = std::cmp::min(buf.len(), self.remains);
        let read_len = self.r.read(&mut buf[..max_len])?;
        self.remains -= read_len;
        Ok(read_len)
    }
}

impl DemuxRead {
    pub fn new(r: Box<dyn Read>) -> DemuxRead {
        DemuxRead { r, remains: 0 }
    }

    /// Return the length of the next real data block.
    ///
    /// Read and print out any messages from the remote end.
    fn read_header_consume_messages(&mut self) -> std::io::Result<usize> {
        loop {
            // Read a length-prefixed packet from peer.
            let mut h = [0u8; 4];
            self.r.read_exact(&mut h)?;

            debug!("got envelope header {}", hex::encode(&h));
            let h = u32::from_le_bytes(h);
            let tag = (h >> 24) as u8;
            let len = (h & 0xffffff) as usize;
            debug!("read envelope tag {} length {:#x}", tag, len);
            if tag == TAG_DATA {
                assert!(len > 0);
                return Ok(len);
            }

            // A message: read and display it here
            let mut message = vec![0; len];
            self.r.read_exact(&mut message)?;
            info!("REMOTE: {}", String::from_utf8_lossy(&message));
            if tag == TAG_FATAL {
                panic!("remote aborted");
            }
        }
    }
}

#[allow(unused)]
pub struct MuxWrite {
    w: Box<dyn Write>,
}

impl MuxWrite {
    #[allow(unused)]
    pub fn new(w: Box<dyn Write>) -> MuxWrite {
        MuxWrite { w }
    }
}

impl Write for MuxWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let l = buf.len();
        assert!(
            l < 0x0ffffff,
            "data length {:#x} is too much for one packet",
            l
        );
        let l: u32 = l as u32 | ((TAG_DATA as u32) << 24);
        let h = l.to_le_bytes();
        self.w
            .write_all(&h)
            .expect("failed to write envelope header");
        self.w
            .write_all(buf)
            .expect("failed to write envelope body");
        debug!("send envelope {}", hex::encode(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}
