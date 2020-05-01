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

//! Statistics/counter structs.

// rsyn: wire-compatible rsync reimplementation in Rust.

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
    pub(crate) fn read(rv: &mut ReadVarint) -> io::Result<ServerStatistics> {
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
