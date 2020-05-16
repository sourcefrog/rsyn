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

//! Length-prefixed, typed, packets multiplexed onto a byte stream.
//!
//! The main function of these is to allow remote error/message strings
//! to be mixed in with normal data transfer.
//!
//! This format is used only from the remote server to the client.

use std::io;
use std::io::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// TODO: Handle other message types from rsync `read_a_msg`.
const TAG_DATA: u8 = 7;
const TAG_FATAL: u8 = 1;

pub struct DemuxRead {
    /// Underlying stream.
    r: Box<dyn Read + Send>,
    /// Amount of data from previous packet remaining to read out.
    current_packet_len: usize,
}

impl Read for DemuxRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
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
    pub fn new(r: Box<dyn Read + Send>) -> DemuxRead {
        DemuxRead {
            r,
            current_packet_len: 0,
        }
    }

    /// Return the length of the next real data block.
    ///
    /// Read and print out any messages from the remote end, without returning
    /// them.
    ///
    /// Returns Ok(0) for a clean EOF before the start of the packet.
    fn read_header_consume_messages(&mut self) -> io::Result<usize> {
        loop {
            // Read a length-prefixed packet from peer.
            let mut h = [0u8; 4];
            if let Err(e) = self.r.read_exact(&mut h) {
                match e.kind() {
                    io::ErrorKind::UnexpectedEof => {
                        debug!("Clean eof before mux packet");
                        return Ok(0);
                    }
                    _ => return Err(e),
                }
            }

            // debug!("got envelope header {{{}}}", hex::encode(&h));
            let h = u32::from_le_bytes(h);
            let tag = (h >> 24) as u8;
            let len = (h & 0xff_ffff) as usize;
            trace!("Read envelope tag {:#04x} length {:#x}", tag, len);
            if tag == TAG_DATA {
                if len == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Zero-length data packet received",
                    ));
                }
                return Ok(len);
            }

            // A human-readable message: read and display it here.
            let mut message = vec![0; len];
            self.r.read_exact(&mut message)?;
            info!("REMOTE: {}", String::from_utf8_lossy(&message).trim_end());
            if tag == TAG_FATAL {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Remote signalled fatal error",
                ));
            }
        }
    }
}

// MAYBE: Add buffering and flushing, so that every single write is
// not sent as a single packet.

/// Translate a stream of bytes into length-prefixed packets.
///
/// This is only used from the server to the client, and
/// at the moment rsyn only acts as a client, so this is never used.
#[allow(unused)]
pub struct MuxWrite {
    w: Box<dyn Write + Send>,
}

impl MuxWrite {
    #[allow(unused)]
    pub fn new(w: Box<dyn Write + Send>) -> MuxWrite {
        MuxWrite { w }
    }
}

impl Write for MuxWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO: Break large buffers into multiple packets instead of erroring.
        let l = buf.len();
        assert!(
            l < 0x0ff_ffff,
            "Data length {:#x} is too much for one packet",
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
        trace!("Send envelope tag {:#x} data {}", l, hex::encode(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}
