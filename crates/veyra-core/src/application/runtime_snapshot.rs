//! P2 Runtime owner supplies live evidence and reads/writes its own last-applied manifest.
//! Business storage does not persist Ready, instance identity or fabricated application versions.
use crate::domain::{ConfigVersion, SelectionVersion};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct InstanceId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeState {
    Stopped,
    Starting {
        instance_id: InstanceId,
    },
    Ready {
        instance_id: InstanceId,
        applied_version: ConfigVersion,
    },
    Recovering {
        instance: Option<AppliedInstance>,
    },
    Failed {
        instance_id: Option<InstanceId>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedInstance {
    pub instance_id: InstanceId,
    pub applied_version: ConfigVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum RuntimeStatus {
    Stopped,
    Starting,
    Ready,
    Recovering,
    Failed,
}

/// Set only after runtime readiness, selection reconciliation and successful manifest commit.
/// A live Ready with a failed manifest commit keeps the preceding last_successful record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LastSuccessfulVersion {
    pub config: ConfigVersion,
    pub selection: SelectionVersion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeFacts {
    pub current: RuntimeState,
    pub last_successful: Option<LastSuccessfulVersion>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub instance_id: Option<InstanceId>,
    pub saved_version: ConfigVersion,
    pub applied_version: Option<ConfigVersion>,
    pub last_successful_version: Option<ConfigVersion>,
    pub last_successful_selection_version: Option<SelectionVersion>,
}
impl RuntimeSnapshot {
    pub fn from_owner(saved_version: ConfigVersion, facts: &RuntimeFacts) -> Self {
        let (status, instance_id, applied_version) = match &facts.current {
            RuntimeState::Stopped => (RuntimeStatus::Stopped, None, None),
            RuntimeState::Starting { instance_id } => {
                (RuntimeStatus::Starting, Some(instance_id.clone()), None)
            }
            RuntimeState::Ready {
                instance_id,
                applied_version,
            } => (
                RuntimeStatus::Ready,
                Some(instance_id.clone()),
                Some(applied_version.clone()),
            ),
            RuntimeState::Recovering { instance } => (
                RuntimeStatus::Recovering,
                instance.as_ref().map(|v| v.instance_id.clone()),
                instance.as_ref().map(|v| v.applied_version.clone()),
            ),
            RuntimeState::Failed { instance_id } => {
                (RuntimeStatus::Failed, instance_id.clone(), None)
            }
        };
        Self {
            status,
            instance_id,
            saved_version,
            applied_version,
            last_successful_version: facts.last_successful.as_ref().map(|v| v.config.clone()),
            last_successful_selection_version: facts
                .last_successful
                .as_ref()
                .map(|v| v.selection.clone()),
        }
    }
    /// No current instance means no applied version; history is not application evidence.
    pub fn has_unapplied_config(&self) -> bool {
        self.applied_version.as_ref() != Some(&self.saved_version)
    }
}
