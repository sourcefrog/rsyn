//! Open a connection to the remote server, and do top-level operations on it.

#![allow(unused_imports)]

use std::io;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::flist::{read_file_list, FileList};
use crate::mux::DemuxRead;
use crate::varint::{ReadVarint, WriteVarint};

const MY_PROTOCOL_VERSION: i32 = 29;

pub struct Connection {
    rv: ReadVarint,
    wv: WriteVarint,

    #[allow(unused)]
    server_version: i32,

    #[allow(unused)]
    salt: i32,

    child: Child,
}

/// Connection to an rsync server.
///
/// Due to the protocol definition, only one transfer (list, send, or receive) can be done per
/// connection.
///
/// TODO: Support other connection modes, especially SSH and daemon.
impl Connection {
    /// Open a new connection to a local rsync subprocess.
    pub fn local_subprocess<P: AsRef<Path>>(path: P) -> io::Result<Connection> {
        let mut child = Command::new("rsync")
            .arg("--server")
            .arg("--sender")
            .arg("-vvr")
            .arg(path.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        // We can ignore the actual child object, although we could keep it
        // if we care about the subprocess exit code.
        let r = Box::new(child.stdout.take().expect("child has no stdout"));
        let w = Box::new(child.stdin.take().expect("child has no stdin"));

        Connection::handshake(r, w, child)
    }

    fn handshake(r: Box<dyn Read>, w: Box<dyn Write>, child: Child) -> io::Result<Connection> {
        let mut wv = WriteVarint::new(w);
        let mut rv = ReadVarint::new(r);

        wv.write_i32(MY_PROTOCOL_VERSION)?;
        let server_version = rv.read_i32().unwrap();
        if server_version < MY_PROTOCOL_VERSION {
            panic!("server protocol version {} is too old", server_version);
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
    pub fn list_files(mut self) -> io::Result<FileList> {
        // send exclusion list length of 0
        self.send_exclusions();
        let file_list = read_file_list(&mut self.rv)?;
        // TODO: With -o, get uid list.
        // TODO: With -g, get gid list.

        // TODO: Only if protocol <30?
        let io_error_count = self.rv.read_i32()?;
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
        info!(
            "server statistics: {:#?}",
            crate::statistics::ServerStatistics::read(&mut self.rv)?
        );

        // one more end?
        self.wv.write_i32(-1)?;
        self.shutdown()?;
        Ok(file_list)
    }

    /// Shut down this connection, consuming the object.
    ///
    /// This isn't the drop method, because it only makes sense to do after
    /// the protocol has reached the natural end.
    fn shutdown(self) -> io::Result<()> {
        let Connection {
            rv,
            wv,
            server_version: _,
            salt: _,
            mut child,
        } = self;

        match rv.check_for_eof() {
            Ok(true) => (),
            Ok(false) => panic!("connection has more input data at shutdown"),
            Err(e) => panic!("unexpected error kind at shutdown: {:?}", e),
        };
        drop(wv);

        // TODO: Should this be returned, somehow?
        // TODO: Should we timeout after a while?
        info!("child process exited with status {}", child.wait()?);

        Ok(())
    }

    fn send_exclusions(&mut self) {
        self.wv.write_i32(0).unwrap();
    }
}
