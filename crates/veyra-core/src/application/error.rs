//! Stable service error contract shared with domain validation.
pub use crate::domain::{AppError, AppErrorCode, ErrorDetail, FieldPath};

impl From<crate::storage::StateStoreError> for AppError {
    fn from(error: crate::storage::StateStoreError) -> Self {
        use crate::storage::StateStoreError as E;
        let detail = match error {
            E::RevisionConflict => return Self::new(AppErrorCode::RevisionConflict),
            E::InvalidState(_) => return Self::validation(FieldPath::Snapshot),
            E::ReadFailed => ErrorDetail::Read,
            E::WriteFailed => ErrorDetail::Write,
            E::ReplaceFailed => ErrorDetail::Replace,
            _ => ErrorDetail::InvalidSnapshot,
        };
        Self::new(AppErrorCode::StorageFailed).with_detail(detail)
    }
}
