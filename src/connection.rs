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

use std::io;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use anyhow::{bail, Context, Result};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::flist::{read_file_list, FileList};
use crate::mux::DemuxRead;
use crate::varint::{ReadVarint, WriteVarint};
use crate::{Options, ServerStatistics};

const MY_PROTOCOL_VERSION: i32 = 29;

/// Connection to an rsync server.
///
/// Connections are obtained from `Address::connect`.
///
/// Due to the protocol definition, only one transfer (list, send, or receive)
/// can be done per connection.
pub struct Connection {
    rv: ReadVarint,
    wv: WriteVarint,

    #[allow(unused)]
    server_version: i32,

    #[allow(unused)]
    salt: i32,

    child: Child,

    #[allow(unused)]
    options: Options,
}

impl Connection {
    /// Start a new connection, by doing the rsync handshake protocol.
    ///
    ///
    /// Most users should call `Address::connect` instead.
    pub(crate) fn handshake(
        r: Box<dyn Read>,
        w: Box<dyn Write>,
        child: Child,
        options: Options,
    ) -> Result<Connection> {
        let mut wv = WriteVarint::new(w);
        let mut rv = ReadVarint::new(r);

        wv.write_i32(MY_PROTOCOL_VERSION)?;
        let server_version = rv.read_i32().unwrap();
        if server_version < MY_PROTOCOL_VERSION {
            bail!("server protocol version {} is too old", server_version);
        }
        // The server and client agree to use the minimum supported version, which will now be
        // ours.

        let salt = rv.read_i32().unwrap();
        debug!(
            "connected to server version {}, salt {:#x}",
            server_version, salt
        );

        // Server-to-client is multiplexed; client-to-server is not.
        // Pull back the underlying stream and wrap it in a demuxed varint
        // encoder.
        let rv = ReadVarint::new(Box::new(DemuxRead::new(rv.take())));

        Ok(Connection {
            rv,
            wv,
            server_version,
            salt,
            child,
            options,
        })
    }

    /// Lists files in the target directory on the server.
    ///
    /// The file list is in the sorted order defined by the protocol, which
    /// is strcmp on the raw bytes of the names.
    pub(crate) fn list_files(mut self) -> Result<(FileList, ServerStatistics)> {
        // send exclusion list length of 0
        self.send_exclusions()?;
        let file_list = read_file_list(&mut self.rv)?;
        // TODO: With -o, get uid list.
        // TODO: With -g, get gid list.

        // TODO: Only if protocol <30?
        let io_error_count = self
            .rv
            .read_i32()
            .context("Failed to read server error count")?;
        if io_error_count > 0 {
            warn!("Server reports {} IO errors", io_error_count);
        }

        // Request no files.
        debug!("send end of phase 1");
        self.wv.write_i32(-1)?; // end of phase 1

        // Server stops here if there were no files.
        if file_list.is_empty() {
            info!("Server returned no files, so we're done");
            self.shutdown()?;
            return Ok((file_list, ServerStatistics::default()));
        }

        assert_eq!(self.rv.read_i32()?, -1);
        debug!("send end of phase 2");
        self.wv.write_i32(-1)?; // end of phase 2
        assert_eq!(self.rv.read_i32()?, -1);
        debug!("send end of sequence");
        self.wv.write_i32(-1)?; // end-of-sequence marker
        assert_eq!(self.rv.read_i32()?, -1);
        let server_stats =
            ServerStatistics::read(&mut self.rv).context("Failed to read server statistics")?;
        info!("server statistics: {:#?}", server_stats);

        // one more end?
        self.wv.write_i32(-1)?;
        self.shutdown()?;
        Ok((file_list, server_stats))
    }

    /// Shut down this connection, consuming the object.
    ///
    /// This isn't the drop method, because it only makes sense to do after
    /// the protocol has reached the natural end.
    fn shutdown(self) -> Result<()> {
        let Connection {
            rv,
            wv,
            server_version: _,
            salt: _,
            mut child,
            options: _,
        } = self;

        rv.check_for_eof()?;
        drop(wv);

        // TODO: Should this be returned, somehow?
        // TODO: Should we timeout after a while?
        let child_result = child.wait()?;
        info!("child process exited with status {}", child_result);

        Ok(())
    }

    fn send_exclusions(&mut self) -> Result<()> {
        self.wv
            .write_i32(0)
            .context("Failed to send exclusion list")
    }
}
