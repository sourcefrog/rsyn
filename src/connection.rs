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

const MY_PROTOCOL_VERSION: i32 = 29;

/// Connection to an rsync server.
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
}

impl Connection {
    /// Open a new connection to a local rsync server subprocess, over a pair of
    /// pipes.
    ///
    /// Since this can only read files off the local filesystem, it's mostly
    /// interesting for testing.
    pub fn local_subprocess<P: AsRef<Path>>(path: P) -> Result<Connection> {
        let mut child = Command::new("rsync")
            .arg("--server")
            .arg("--sender")
            .arg("-vvr")
            .arg(path.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to launch rsync subprocess")?;

        // We can ignore the actual child object, although we could keep it
        // if we care about the subprocess exit code.
        let r = Box::new(child.stdout.take().expect("child has no stdout"));
        let w = Box::new(child.stdin.take().expect("child has no stdin"));

        Connection::handshake(r, w, child)
    }

    fn handshake(r: Box<dyn Read>, w: Box<dyn Write>, child: Child) -> Result<Connection> {
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
        })
    }

    /// Return a list of files from the server.
    ///
    /// The file list is in the sorted order defined by the protocol, which
    /// is strcmp on the raw bytes of the names.
    pub fn list_files(mut self) -> Result<FileList> {
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
            warn!("server reports {} IO errors", io_error_count);
        }

        // Request no files.
        self.wv.write_i32(-1)?; // end of phase 1
        assert_eq!(self.rv.read_i32()?, -1);
        self.wv.write_i32(-1)?; // end of phase 2
        assert_eq!(self.rv.read_i32()?, -1);
        self.wv.write_i32(-1)?; // end-of-sequence marker
        assert_eq!(self.rv.read_i32()?, -1);
        // TODO: Return the statistics.
        let server_stats = crate::statistics::ServerStatistics::read(&mut self.rv)
            .context("Failed to read server statistics")?;
        info!("server statistics: {:#?}", server_stats);

        // one more end?
        self.wv.write_i32(-1)?;
        self.shutdown()?;
        Ok(file_list)
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
