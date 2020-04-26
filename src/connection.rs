#![allow(unused_imports)]

use std::io;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::process::{Command, Stdio};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::flist::read_file_list;
use crate::mux::DemuxRead;
use crate::proto::{ReadProto, WriteProto};

const MY_PROTOCOL_VERSION: i32 = 29;

pub struct Connection {
    r: Box<dyn Read>,
    w: Box<dyn Write>,
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

    fn handshake(mut r: Box<dyn Read>, mut w: Box<dyn Write>) -> io::Result<Connection> {
        w.write_i32(MY_PROTOCOL_VERSION)?;

        let server_version = r.read_i32().unwrap();
        assert_eq!(
            server_version, MY_PROTOCOL_VERSION,
            "server version {} not supported?",
            server_version
        );
        let salt = r.read_i32().unwrap();
        debug!(
            "connected to server version {}, salt {:#x}",
            server_version, salt
        );

        Ok(Connection {
            // Server-to-client is multiplexed; client-to-server is not.
            r: Box::new(DemuxRead::new(r)),
            w,
            server_version,
            salt,
        })
    }

    pub fn list_files(&mut self) -> io::Result<()> {
        // send exclusion list length of 0
        self.send_exclusions();
        for e in read_file_list(&mut self.r)?.iter() {
            println!("{}", String::from_utf8_lossy(&e.name));
        }
        // TODO: With -o, get uid list.
        // TODO: With -g, get gid list.

        // TODO: Only if protocol <30?
        let io_error_count = self.r.read_i32()?;
        info!("server reports IO errors on {} files", io_error_count);

        // Request no files.
        self.w.write_i32(-1)?; // end of phase 1
        assert_eq!(self.r.read_i32()?, -1);
        self.w.write_i32(-1)?; // end of phase 2
        assert_eq!(self.r.read_i32()?, -1);
        self.w.write_i32(-1)?; // end-of-sequence marker
        assert_eq!(self.r.read_i32()?, -1);
        info!("server statistics: {:#?}", self.r.read_server_statistics()?);

        // one more end?
        self.w.write_i32(-1)?;
        loop {
            dbg!(self.r.read_u8()?);
        }
        // Ok(())
    }

    fn send_exclusions(&mut self) {
        self.w.write_i32(0).unwrap();
    }
}
