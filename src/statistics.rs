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

/// Description of what happened during a transfer.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Summary {
    /// Server reported this many errors while building the file count.
    /// (Typically, "permission denied" on a subdirectory.)
    pub server_flist_io_error_count: i32,

    /// Statistics sent from the server.
    pub server_stats: crate::ServerStatistics,

    /// If a child process was used for the connection and it has exited,
    /// it's exit status.
    pub child_exit_status: Option<std::process::ExitStatus>,

    /// Number of invalid file indexes received. Should be 0.
    pub invalid_file_index_count: usize,

    /// Number of times the whole-file MD4 did not match.
    pub whole_file_sum_mismatch_count: usize,

    /// Number of literal bytes (rather than references to the old file) received.
    pub literal_bytes_received: usize,

    /// Number of files received.
    pub files_received: usize,
}

/// Statistics from a remote server about how much work it did.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
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
