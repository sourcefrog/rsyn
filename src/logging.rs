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

//! Log helper functions.

use std::fmt;

/// Format a `log::Record`.
///
/// This is exposed just as a convenience for tests or other users. Any logger
/// configuration should work.
pub fn format_log(out: fern::FormatCallback, args: &fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
        "[{:<30}][{}] {}",
        record.target(),
        record.level().to_string().chars().next().unwrap(),
        args
    ))
}
