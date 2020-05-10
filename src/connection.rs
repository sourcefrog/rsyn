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

//! A connection to an rsync server.

#![allow(unused_imports)]

use std::convert::TryInto;
use std::io;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use anyhow::{bail, Context, Result};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::flist::{read_file_list, FileEntry, FileList};
use crate::mux::DemuxRead;
use crate::varint::{ReadVarint, WriteVarint};
use crate::{Options, ServerStatistics};

const MY_PROTOCOL_VERSION: i32 = 27;

/// Connection to an rsync server.
///
/// Due to the protocol definition, only one transfer (list, send, or receive)
/// can be done per connection.
pub(crate) struct Connection {
    rv: ReadVarint,
    wv: WriteVarint,

    /// Mutually-agreed rsync protocol version number.
    protocol_version: i32,

    #[allow(unused)]
    salt: i32,

    child: Child,

    #[allow(unused)]
    options: Options,
}

impl Connection {
    /// Start a new connection, by doing the rsync handshake protocol.
    ///
    /// The public interface is through `Client`.
    pub(crate) fn handshake(
        r: Box<dyn Read>,
        w: Box<dyn Write>,
        child: Child,
        options: Options,
    ) -> Result<Connection> {
        let mut wv = WriteVarint::new(w);
        let mut rv = ReadVarint::new(r);

        wv.write_i32(MY_PROTOCOL_VERSION)?;
        let remote_protocol_version = rv.read_i32().unwrap();
        if remote_protocol_version < MY_PROTOCOL_VERSION {
            bail!(
                "server protocol version {} is too old",
                remote_protocol_version
            );
        }
        // The server and client agree to use the minimum supported version, which will now be
        // ours.

        let salt = rv.read_i32().unwrap();
        debug!(
            "Connected to server version {}, salt {:#x}",
            remote_protocol_version, salt
        );
        let protocol_version = std::cmp::min(MY_PROTOCOL_VERSION, remote_protocol_version);
        debug!("Agreed protocol version {}", protocol_version);

        // Server-to-client is multiplexed; client-to-server is not.
        // Pull back the underlying stream and wrap it in a demuxed varint
        // encoder.
        let rv = ReadVarint::new(Box::new(DemuxRead::new(rv.take())));

        Ok(Connection {
            rv,
            wv,
            protocol_version,
            salt,
            child,
            options,
        })
    }

    /// Lists files in the target directory on the server.
    ///
    /// The file list is in the sorted order defined by the protocol, which
    /// is strcmp on the raw bytes of the names.
    pub(crate) fn list_files(self) -> Result<(FileList, ServerStatistics)> {
        self.receive()
    }

    fn receive(mut self) -> Result<(FileList, ServerStatistics)> {
        // Analogous to rsync/receiver.c recv_files().
        // let max_phase = if self.protocol_version >= 29 { 2 } else { 1 };
        let max_phase = 2;

        // send exclusion list length of 0
        self.send_exclusions()?;
        let mut file_list = read_file_list(&mut self.rv)?;
        crate::flist::sort(&mut file_list);
        // TODO: With -o, get uid list.
        // TODO: With -g, get gid list.

        if self.protocol_version < 30 {
            let io_error_count = self
                .rv
                .read_i32()
                .context("Failed to read server error count")?;
            if io_error_count > 0 {
                // TODO: Somehow make this, and other soft errors, observable to the API client.
                warn!("Server reports {} IO errors", io_error_count);
            }
        }

        for phase in 1..=max_phase {
            debug!("Start phase {}", phase);

            // Server stops here if there were no files.
            if file_list.is_empty() {
                info!("Server returned no files, so we're done");
                self.shutdown()?;
                return Ok((file_list, ServerStatistics::default()));
            }

            // Select the files we want to receive.
            // self.generate_one_file(&file_list)?;

            self.wv
                .write_i32(-1)
                .context("Failed to send phase transition")?; // end of phase 1

            // Server sends a series of file indexes, terminated by -1.
            loop {
                let idx = self.rv.read_i32().context("Failed to read file index")?;
                trace!("Received index {}", idx);
                if idx == -1 {
                    break;
                }
                // TODO: Receive and act on the per-file message.
            }
        }

        debug!("Send end of sequence");
        self.wv
            .write_i32(-1)
            .context("Failed to send end-of-sequence marker")?;
        // TODO: In later versions (which?) read an end-of-sequence marker?
        let server_stats = self
            .read_server_statistics()
            .context("Failed to read server statistics")?;
        info!("{:#?}", server_stats);

        // TODO: In later versions, send a final -1 marker.
        self.shutdown()?;
        Ok((file_list, server_stats))
    }

    /// Produce a generator request for the first file in the list, as if it
    /// doesn't exist locally.
    fn generate_one_file(&mut self, file_list: &[FileEntry]) -> Result<()> {
        // compare to `recv_generator` in generator.c.

        assert!(!file_list.is_empty());
        for (idx, entry) in file_list.iter().filter(|e| e.is_file()).enumerate().take(1) {
            debug!(
                "Send request for file idx {}, name {:?}",
                idx,
                entry.name_lossy_string()
            );
            self.wv.write_i32(idx.try_into().unwrap())?; // index

            // Like write_sum_head.
            self.wv.write_i32(0)?; // count
            self.wv.write_i32(0)?; // blength
            self.wv.write_i32(0)?; // s2length
            self.wv.write_i32(0)?; // remainder
        }

        Ok(())
    }

    /// Shut down this connection, consuming the object.
    ///
    /// This isn't the drop method, because it only makes sense to do after
    /// the protocol has reached the natural end.
    fn shutdown(self) -> Result<()> {
        let Connection {
            rv,
            wv,
            protocol_version: _,
            salt: _,
            mut child,
            options: _,
        } = self;

        rv.check_for_eof()?;
        drop(wv);

        // TODO: Should this be returned, somehow?
        // TODO: Should we timeout after a while?
        // TODO: Map rsync return codes to messages.
        let child_result = child.wait()?;
        info!("Child process exited: {}", child_result);

        Ok(())
    }

    fn send_exclusions(&mut self) -> Result<()> {
        self.wv
            .write_i32(0)
            .context("Failed to send exclusion list")
    }

    fn read_server_statistics(&mut self) -> Result<ServerStatistics> {
        Ok(ServerStatistics {
            total_bytes_read: self.rv.read_i64()?,
            total_bytes_written: self.rv.read_i64()?,
            total_file_size: self.rv.read_i64()?,
            flist_build_time: if self.protocol_version >= 29 {
                Some(self.rv.read_i64()?)
            } else {
                None
            },
            flist_xfer_time: if self.protocol_version >= 29 {
                Some(self.rv.read_i64()?)
            } else {
                None
            },
        })
    }
}
