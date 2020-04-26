#![allow(unused_imports)]

use std::io;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::process::{Command, Stdio};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::flist::read_file_list;
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
}

impl Connection {
    pub fn local_subprocess(path: &str) -> io::Result<Connection> {
        let mut child = Command::new("rsync")
            .arg("--server")
            .arg("--sender")
            .arg("-vvr")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        // We can ignore the actual child object, although we could keep it
        // if we care about the subprocess exit code.
        let r = Box::new(child.stdout.take().expect("child has no stdout"));
        let w = Box::new(child.stdin.take().expect("child has no stdin"));

        Connection::handshake(r, w)
    }

    fn handshake(r: Box<dyn Read>, w: Box<dyn Write>) -> io::Result<Connection> {
        let mut wv = WriteVarint::new(w);
        let mut rv = ReadVarint::new(r);

        wv.write_i32(MY_PROTOCOL_VERSION)?;

        let server_version = rv.read_i32().unwrap();
        assert_eq!(
            server_version, MY_PROTOCOL_VERSION,
            "server version {} not supported?",
            server_version
        );
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
        })
    }

    pub fn list_files(&mut self) -> io::Result<()> {
        // send exclusion list length of 0
        self.send_exclusions();
        for e in read_file_list(&mut self.rv)?.iter() {
            println!("{}", String::from_utf8_lossy(&e.name));
        }
        // TODO: With -o, get uid list.
        // TODO: With -g, get gid list.

        // TODO: Only if protocol <30?
        let io_error_count = self.rv.read_i32()?;
        info!("server reports IO errors on {} files", io_error_count);

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
        loop {
            dbg!(self.rv.read_u8()?);
        }
        // Ok(())
    }

    fn send_exclusions(&mut self) {
        self.wv.write_i32(0).unwrap();
    }
}
