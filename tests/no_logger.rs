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

use rsyn::Client;

/// Check that we're not counting on the side effects of any logging.
///
/// This must be in a different target from other interop tests, so that it
/// runs in a different process, and doesn't accidentally inherit a global
/// logger.
#[test]
fn list_files_with_no_logger() {
    // TODO: Assertions about the contents.
    // TODO: Assert that there is, in fact, no logger.
    Client::local("./src")
        .list_files()
        .expect("Failed to list files");
}
