use std::io::prelude::*;
use std::process::{Command, Stdio};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::parser;

const MY_PROTOCOL_VERSION: u32 = 29;

pub struct Connection {
    r: Box<dyn Read>,
    w: Box<dyn Write>,
    server_version: u32,
    salt: u32,
}

impl Connection {
    pub fn local_subprocess() -> Connection {
        let mut child = Command::new("rsync")
            .arg("--server")
            .arg(".")
            .arg("/etc")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to launch rsync");

        // We can ignore the actual child object, although we could keep it
        // if we care about the subprocess exit code.
        let r = Box::new(child.stdout.take().expect("child has no stdout"));
        let w = Box::new(child.stdin.take().expect("child has no stdin"));

        Connection::handshake(r, w)
    }

    fn handshake(mut r: Box<dyn Read>, mut w: Box<dyn Write>) -> Connection {
        send_u32(&mut w, MY_PROTOCOL_VERSION)
            .expect("failed to send version to child");

        let mut b = [0u8; 8];
        r.read_exact(&mut b).unwrap();
        let (rest, (server_version, salt)) = parser::server_greeting(&b).unwrap();
        assert!(rest.is_empty());
        debug!("connected to server version {}, salt {:#x}", server_version, salt);

        Connection {
            r,
            w,
            server_version,
            salt,
        }
    }
}

fn send_u32(w: &mut dyn Write, v: u32) -> std::io::Result<()> {
    let b = v.to_le_bytes();
    w.write_all(&b)
}

/// Handles transmission of length-prefixed envelope packets.
struct Mux {
}