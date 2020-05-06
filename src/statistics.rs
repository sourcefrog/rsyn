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

/// Statistics from a remote server about how much work it did.
#[derive(Debug, Default)]
pub struct ServerStatistics {
    // The rsync(1) man page has some description of these.
    /// Total bytes sent over the network from the client to the server.
    pub total_bytes_read: i64,
    /// Total bytes sent over the network from the server to the client,
    /// ignoring any text messages.
    pub total_bytes_written: i64,
    /// The sum of the size of all file sizes in the transfer. This does not
    /// count directories or special files, but does include the size of
    /// symlinks.
    pub total_file_size: i64,
    /// The number of seconds spent by the server building a file list.
    pub flist_build_time: Option<i64>,
    /// The number of seconds the server spent sending the file list to the
    /// client.
    pub flist_xfer_time: Option<i64>,
    // TODO: More fields in at least some protocol versions.
}
