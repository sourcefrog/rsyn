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
use crossbeam::thread;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use md4::{Digest, Md4};

use crate::flist::{read_file_list, FileEntry, FileList};
use crate::mux::DemuxRead;
use crate::sums::SumHead;
use crate::varint::{ReadVarint, WriteVarint};
use crate::{LocalTree, Options, ServerStatistics, Summary};

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

    /// Permutation to checksums, pushed as a le i32 at the start of file MD4s.
    checksum_seed: i32,

    /// The child process carrying this connection.
    child: Child,

    /// Connection options, corresponding to a subset of rsync command-line options.
    ///
    /// The options affect which fields are present or not on the wire.
    options: Options,
}

impl Connection {
    /// Start a new connection, by doing the rsync handshake protocol.
    ///
    /// The public interface is through `Client`.
    pub(crate) fn handshake(
        r: Box<dyn Read + Send>,
        w: Box<dyn Write + Send>,
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
        // The server and client agree to use the minimum supported version,
        // which will now be ours, because we refuse to accept anything
        // older.

        let checksum_seed = rv.read_i32().unwrap();
        debug!(
            "Connected to server version {}, checksum_seed {:#x}",
            remote_protocol_version, checksum_seed
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
            checksum_seed,
            child,
            options,
        })
    }

    /// Receive files from the server to the given LocalTree.
    pub fn receive(mut self, local_tree: &mut LocalTree) -> Result<(FileList, Summary)> {
        // Analogous to rsync/receiver.c recv_files().
        // let max_phase = if self.protocol_version >= 29 { 2 } else { 1 };
        let max_phase = 2;
        let mut summary = Summary::default();

        send_empty_exclusions(&mut self.wv)?;
        let file_list = read_file_list(&mut self.rv)?;
        // TODO: With -o, get uid list.
        // TODO: With -g, get gid list.

        if self.protocol_version < 30 {
            let io_error_count = self
                .rv
                .read_i32()
                .context("Failed to read server error count")?;
            if io_error_count > 0 {
                warn!("Server reports {} IO errors", io_error_count);
            }
            summary.server_flist_io_error_count = io_error_count;
        }

        // Server stops here if there were no files.
        if file_list.is_empty() {
            info!("Server returned no files, so we're done");
            // TODO: Maybe write one -1 here?
            self.shutdown(&mut summary)?;
            return Ok((file_list, summary));
        }

        for phase in 1..=max_phase {
            debug!("Start phase {}", phase);
            if phase == 1 && !self.options.list_only {
                self.receive_files(&file_list, local_tree, &mut summary)?;
            } else {
                self.wv
                    .write_i32(-1)
                    .context("Failed to send phase transition")?;
                assert_eq!(self.rv.read_i32()?, -1);
            }
        }

        debug!("Send end of sequence");
        self.wv
            .write_i32(-1)
            .context("Failed to send end-of-sequence marker")?;
        // TODO: In later versions (which?) read an end-of-sequence marker?
        summary.server_stats = read_server_statistics(&mut self.rv, self.protocol_version)
            .context("Failed to read server statistics")?;

        // TODO: In later versions, send a final -1 marker.
        self.shutdown(&mut summary)?;
        info!("{:#?}", summary);
        Ok((file_list, summary))
    }

    /// Download all regular files.
    ///
    /// Includes sending requests for them (with no basis) and receiving the data.
    fn receive_files(
        &mut self,
        file_list: &[FileEntry],
        local_tree: &mut LocalTree,
        summary: &mut Summary,
    ) -> Result<()> {
        // compare to `recv_generator` in generator.c.
        assert!(!file_list.is_empty());
        let rv = &mut self.rv;
        let wv = &mut self.wv;
        let checksum_seed = self.checksum_seed;
        thread::scope(|scope| {
            scope
                .builder()
                .name("rsyn_receiver".to_owned())
                .spawn(|_| receive_offered_files(rv, checksum_seed, file_list, local_tree, summary))
                .expect("Failed to spawn receiver thread");
            generate_files(wv, file_list).unwrap();
        })
        .unwrap();
        debug!("receive_files done");
        Ok(()) // TODO: Handle errors from threads correctly
    }

    /// Shut down this connection, consuming the object.
    ///
    /// This isn't the drop method, because it only makes sense to do after
    /// the protocol has reached the natural end.
    fn shutdown(self, summary: &mut Summary) -> Result<()> {
        let Connection {
            rv,
            wv,
            protocol_version: _,
            checksum_seed: _,
            mut child,
            options: _,
        } = self;

        rv.check_for_eof()?;
        drop(wv);

        // TODO: Should we timeout after a while?
        // TODO: Map rsync return codes to messages.
        let child_exit_status = child.wait()?;
        summary.child_exit_status = Some(child_exit_status);
        info!("Child process exited: {}", child_exit_status);

        Ok(())
    }
}

fn read_server_statistics(rv: &mut ReadVarint, protocol_version: i32) -> Result<ServerStatistics> {
    Ok(ServerStatistics {
        total_bytes_read: rv.read_i64()?,
        total_bytes_written: rv.read_i64()?,
        total_file_size: rv.read_i64()?,
        flist_build_time: if protocol_version >= 29 {
            Some(rv.read_i64()?)
        } else {
            None
        },
        flist_xfer_time: if protocol_version >= 29 {
            Some(rv.read_i64()?)
        } else {
            None
        },
    })
}

fn send_empty_exclusions(wv: &mut WriteVarint) -> Result<()> {
    wv.write_i32(0).context("Failed to send exclusion list")
}

fn generate_files(wv: &mut WriteVarint, file_list: &[FileEntry]) -> Result<()> {
    for (idx, entry) in file_list.iter().enumerate().filter(|(_idx, e)| e.is_file()) {
        debug!(
            "Send request for file idx {}, name {:?}",
            idx,
            entry.name_lossy_string()
        );
        wv.write_i32(idx.try_into().unwrap())?;
        SumHead::zero().write(wv)?;
    }
    debug!("Generator done");
    wv.write_i32(-1)
        .context("Failed to send phase transition")?;
    Ok(())
}

/// Receive files from the sender until it sends an end-of-phase marker.
fn receive_offered_files(
    rv: &mut ReadVarint,
    checksum_seed: i32,
    file_list: &[FileEntry],
    local_tree: &mut LocalTree,
    summary: &mut Summary,
) -> Result<()> {
    // Files normally return in the order the receiver requests them, but this isn't guaranteed.
    // And if the sender fails to open the file, it just doesn't send any message, it just
    // continues to the next one.
    loop {
        let remote_idx = rv.read_i32()?;
        if remote_idx == -1 {
            debug!("Received end-of-phase marker");
            return Ok(());
        }
        let idx = remote_idx as usize;
        if idx >= file_list.len() {
            summary.invalid_file_index_count += 1;
            error!("Remote file index {} is out of range", remote_idx)
        }
        receive_file(rv, checksum_seed, &file_list[idx], local_tree, summary)?;
        summary.files_received += 1;
    }
}

fn receive_file(
    rv: &mut ReadVarint,
    checksum_seed: i32,
    entry: &FileEntry,
    _local_tree: &LocalTree,
    summary: &mut Summary,
) -> Result<()> {
    // Like |receive_data|.
    let name = entry.name_lossy_string();
    info!("Receive {:?}", name);
    let sums = SumHead::read(rv)?;
    trace!("Got sums for {:?}: {:?}", name, sums);
    let mut hasher = Md4::new();
    hasher.input(checksum_seed.to_le_bytes());
    loop {
        // TODO: Specially handle data for deflate mode.
        // Like rsync |simple_recv_token|.
        let t = rv.read_i32()?;
        if t == 0 {
            break;
        } else if t < 0 {
            todo!("Block copy reference")
        } else {
            let t = t.try_into().unwrap();
            let content = rv.read_byte_string(t)?;
            assert_eq!(content.len(), t);
            summary.literal_bytes_received += content.len();
            hasher.input(content);
            // TODO: Write it to the local tree.
        }
    }
    let remote_md4 = rv.read_byte_string(crate::MD4_SUM_LENGTH)?;
    let local_md4 = hasher.result();
    if local_md4[..] != remote_md4[..] {
        // TODO: Remember the error, but don't bail out. Try again in phase 2.
        summary.whole_file_sum_mismatch_count += 1;
        error!(
            "MD4 mismatch for {:?}: sender {}, receiver {}",
            name,
            hex::encode(remote_md4),
            hex::encode(local_md4)
        );
    } else {
        debug!(
            "Completed file {:?} with matching MD4 {}",
            name,
            hex::encode(&remote_md4)
        );
    }
    Ok(())
}
