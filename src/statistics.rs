// rsyn: wire-compatible rsync reimplementation in Rust.

//! Statistics/counter structs.

use std::io;

use crate::varint::ReadVarint;

#[derive(Debug)]
pub struct ServerStatistics {
    pub total_bytes_read: i64,
    pub total_bytes_written: i64,
    pub total_file_size: i64,
    pub flist_build_time: i64,
    pub flist_xfer_time: i64,
}

impl ServerStatistics {
    pub fn read(rv: &mut ReadVarint) -> io::Result<ServerStatistics> {
        // TODO: Perhaps this should be part of the Connection, or some place
        // that knows the protocol version.
        Ok(ServerStatistics {
            total_bytes_read: rv.read_i64()?,
            total_bytes_written: rv.read_i64()?,
            total_file_size: rv.read_i64()?,
            // TODO: These last two are only set for protocol >=29.
            flist_build_time: rv.read_i64()?,
            flist_xfer_time: rv.read_i64()?,
        })
    }
}
