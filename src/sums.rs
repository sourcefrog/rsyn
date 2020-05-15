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

//! A collection of strong and weak sums for a single file, from which deltas
//! can be generated.

use crate::varint::{ReadVarint, WriteVarint};
use crate::Result;

#[derive(Debug)]
pub(crate) struct SumHead {
    // like rsync |sum_struct|.
    count: i32,
    blength: i32,
    s2length: i32,
    remainder: i32,
}

impl SumHead {
    /// Create an empty SumHead describing an empty or absent file.
    pub(crate) fn zero() -> Self {
        SumHead {
            count: 0,
            blength: 0,
            s2length: 0,
            remainder: 0,
        }
    }

    pub fn read(rv: &mut ReadVarint) -> Result<Self> {
        // TODO: Encoding varies per protocol version.
        // TODO: Assertions about the values?
        Ok(SumHead {
            count: rv.read_i32()?,
            blength: rv.read_i32()?,
            s2length: rv.read_i32()?,
            remainder: rv.read_i32()?,
        })
    }

    pub fn write(&self, wv: &mut WriteVarint) -> Result<()> {
        wv.write_i32(self.count)?;
        wv.write_i32(self.blength)?;
        wv.write_i32(self.s2length)?;
        wv.write_i32(self.remainder)?;
        Ok(())
    }
}
