// rsyn: wire-compatible rsync reimplementation in Rust.

//! Statistics/counter structs.

use std::io;
use std::io::prelude::*;

use crate::proto::{ReadProto, WriteProto};

#[derive(Debug)]
pub struct ServerStatistics {
    pub total_bytes_read: i64,
    pub total_bytes_written: i64,
    pub total_file_size: i64,
    pub flist_build_time: i64,
    pub flist_xfer_time: i64,
}

impl ServerStatistics {
    pub fn read(r: &mut (dyn Read + 'static)) -> io::Result<ServerStatistics> {
        // TODO: Perhaps this should be part of the Connection, or some place
        // that knows the protocol version.
        Ok(ServerStatistics {
            total_bytes_read: r.read_i64()?,
            total_bytes_written: r.read_i64()?,
            total_file_size: r.read_i64()?,
            // TODO: These last two are only set for protocol >=29.
            flist_build_time: r.read_i64()?,
            flist_xfer_time: r.read_i64()?,
        })
    }
}