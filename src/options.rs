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

//! Command-line options controlling the local and remote processes.

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Options {
    /// Recurse into directories.
    pub recursive: bool,

    /// Command to run to start the rsync server, typically remotely.
    ///
    /// May be multiple words, which will be passed as separate shell arguments.
    ///
    /// If unset, just "rsync".
    pub rsync_command: Option<Vec<String>>,

    /// Command to open a connection to the remote server.
    ///
    /// May be multiple words to include options, which will be passed as separate
    /// shell arguments.
    ///
    /// If unset, just "ssh".
    pub ssh_command: Option<Vec<String>>,

    /// Only list files.
    ///
    /// This is implied by `Address:list_files` and need not be separately set.
    pub list_only: bool,

    /// Be verbose.
    ///
    /// (This is passed to the server to encourage it to be verbose too.)
    pub verbose: u32,
}
