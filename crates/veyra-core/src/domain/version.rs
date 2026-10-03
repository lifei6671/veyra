use serde::{Deserialize, Serialize};

use super::{AppError, AppErrorCode};

/// One identity for the whole business snapshot. Revisions have no ordering across epochs.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct StateEpoch([u8; 16]);

impl StateEpoch {
    pub fn fresh() -> Result<Self, AppError> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(|_| AppError::new(AppErrorCode::StorageFailed))?;
        Ok(Self(bytes))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StateVersion {
    pub epoch: StateEpoch,
    pub revision: u64,
}

impl StateVersion {
    pub fn compare(&self, other: &Self) -> Result<std::cmp::Ordering, AppError> {
        if self.epoch != other.epoch {
            return Err(AppError::new(AppErrorCode::RevisionConflict));
        }
        Ok(self.revision.cmp(&other.revision))
    }

    pub fn require(&self, expected: &Self) -> Result<(), AppError> {
        if self.compare(expected)? != std::cmp::Ordering::Equal {
            return Err(AppError::new(AppErrorCode::RevisionConflict));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConfigVersion(pub StateVersion);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SelectionVersion(pub StateVersion);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SnapshotVersion {
    pub config: ConfigVersion,
    pub selection: SelectionVersion,
}
