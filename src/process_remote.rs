mod lldb;

#[derive(Debug, thiserror::Error)]
pub enum ProcessRemoteError {
    #[error("byte order mismatch")]
    ByteOrderMismatch,
    #[error("pointer size mismatch")]
    PointerSizeMismatch,
    // including internal data error or loading image error
    #[error("failed to get from process")]
    FailedToGetFromProcess { reason: String },
    #[error("non-utf8 log contents")]
    NonUtf8LogContents,
}

pub(crate) fn base_err(reason: impl ToString) -> ProcessRemoteError {
    ProcessRemoteError::FailedToGetFromProcess {
        reason: reason.to_string(),
    }
}

pub use lldb::get_buffer;
pub type ProcessId = ::lldb::lldb_pid_t;
