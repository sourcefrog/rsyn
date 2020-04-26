use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

const TAG_DATA: u8 = 7;
const TAG_FATAL: u8 = 1;

pub struct DemuxRead {
    r: Box<dyn Read>,
    /// Amount of data from previous packet remaining to read out
    current_packet_len: usize,
}

impl Read for DemuxRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.current_packet_len == 0 {
            self.current_packet_len = self.read_header_consume_messages()?;
        }
        let max_len = std::cmp::min(buf.len(), self.current_packet_len);
        let read_len = self.r.read(&mut buf[..max_len])?;
        self.current_packet_len -= read_len;
        Ok(read_len)
    }
}

impl DemuxRead {
    /// Construct a new packet demuxer, wrapping an underlying Read (typically
    /// a pipe).
    pub fn new(r: Box<dyn Read>) -> DemuxRead {
        DemuxRead { r, current_packet_len: 0 }
    }

    /// Return the length of the next real data block.
    ///
    /// Read and print out any messages from the remote end, without returning
    /// them.
    ///
    /// Returns Ok(0) for a clean EOF before the start of the packet.
    fn read_header_consume_messages(&mut self) -> std::io::Result<usize> {
        loop {
            // Read a length-prefixed packet from peer.
            let mut h = [0u8; 4];
            if let Err(e) = self.r.read_exact(&mut h) {
                match e.kind() {
                    io::ErrorKind::UnexpectedEof => {
                        debug!("clean eof before mux packet");
                        return Ok(0);
                    }
                    _ => return Err(e),
                }
            }

            // debug!("got envelope header {{{}}}", hex::encode(&h));
            let h = u32::from_le_bytes(h);
            let tag = (h >> 24) as u8;
            let len = (h & 0xffffff) as usize;
            debug!("read envelope tag {:#02x} length {:#x}", tag, len);
            if tag == TAG_DATA {
                assert!(len > 0);
                return Ok(len);
            }

            // A human-readable message: read and display it here.
            let mut message = vec![0; len];
            self.r.read_exact(&mut message)?;
            info!("REMOTE: {}", String::from_utf8_lossy(&message).trim_end());
            if tag == TAG_FATAL {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "remote signalled fatal error",
                ));
            }
        }
    }
}


// TODO: Maybe add buffering and flushing, so that every single write is
// not sent as a single packet.

/// Translate a stream of bytes into length-prefixed packets.
///
/// This is only used from the server to the client, and
/// at the moment rsyn only acts as a client, so this is never used.
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
