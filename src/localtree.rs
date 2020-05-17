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

//! Facade for local-filesystem operations.

use std::path::{Path, PathBuf};

use anyhow::Context;
use tempfile::NamedTempFile;

use crate::Result;

/// A filesystem tree local to this process.
///
/// The local tree is the destination for downloads and the source for uploads.
///
/// All local IO is funneled through this layer so that it can be observed
/// and so filenames can be checked. (And perhaps later, applications can provide
/// new implementations that don't literally use the local filesystem.)
pub struct LocalTree {
    root: PathBuf,
}

/// A file being written into the local tree.
///
/// It becomes visible under its name only when it's persisted.
#[derive(Debug)]
pub struct WriteFile {
    final_path: PathBuf,
    temp: NamedTempFile,
}

impl LocalTree {
    /// Construct a new LocalTree addressing a local directory.
    pub fn new<P: Into<PathBuf>>(root: P) -> LocalTree {
        LocalTree { root: root.into() }
    }

    /// Open a file for write.
    ///
    /// The result, a `WriteFile` can be used as `std::io::Write`, but must then be finalized
    /// before the results are committed to the final file name.
    ///
    /// `path` is the relative path.
    pub fn write_file<P: AsRef<Path>>(&self, path: &P) -> Result<WriteFile> {
        let final_path = self.root.join(path.as_ref());
        // Store the temporary file in its subdirectory, not in the root.
        let temp = NamedTempFile::new_in(final_path.parent().unwrap())?;
        Ok(WriteFile { final_path, temp })
    }
}

impl WriteFile {
    /// Finish writing to this file and store it to its permanent location.
    pub fn finalize(self) -> Result<()> {
        let WriteFile { temp, final_path } = self;
        temp.persist(&final_path)
            .with_context(|| format!("Failed to persist temporary file to {:?}", final_path))?;
        Ok(())
    }

    /// The full path to which this file will eventually be written.
    pub fn final_path(&self) -> &Path {
        &self.final_path
    }
}

impl std::io::Write for WriteFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.temp.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.temp.flush()
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.temp.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.temp.write_all(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn write_a_file() {
        let tempdir = tempfile::Builder::new()
            .prefix("rsyn_localtree_write_a_file")
            .tempdir()
            .unwrap();
        let lt = LocalTree::new(tempdir.path());
        let mut f = lt.write_file(&"hello").unwrap();
        let final_path = tempdir.path().join("hello");

        // File does not yet exist until it's finalized.
        assert!(fs::metadata(&final_path).is_err());

        writeln!(f, "The answer is: {}", 42).unwrap();
        f.finalize().unwrap();
        assert!(fs::metadata(&final_path).unwrap().is_file());
        let mut content = String::new();
        File::open(&final_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "The answer is: 42\n");
    }

    #[test]
    fn dropped_file_is_discarded() {
        let tempdir = tempfile::Builder::new()
            .prefix("rsyn_localtree_dropped_file_is_discarded")
            .tempdir()
            .unwrap();
        let lt = LocalTree::new(tempdir.path());
        let mut f = lt.write_file(&"hello").unwrap();
        let final_path = f.final_path().to_owned();
        f.write_all("some content".as_bytes()).unwrap();
        drop(f);
        // File does not yet exist
        assert!(fs::metadata(&final_path).is_err());
        // Can also drop the LocalTree but not the tempdir, and the file still
        // does not exist.
        drop(lt);
        assert!(fs::metadata(tempdir.path()).unwrap().is_dir());
        assert!(fs::metadata(&final_path).is_err());
    }
}
