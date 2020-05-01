use rsyn::Connection;

/// Check that we're not counting on the side effects of any logging.
///
/// This must be in a different target from other interop tests, so that it
/// runs in a different process, and doesn't accidentally inherit a global
/// logger.
#[test]
fn list_files_with_no_logger() {
    // TODO: Assertions about the contents.
    Connection::local_subprocess("./src")
        .expect("Failed to connect")
        .list_files()
        .expect("Failed to list files");
}
