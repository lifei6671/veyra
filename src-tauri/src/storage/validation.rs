use crate::domain::{AppState, StateValidationError};

use super::StateStoreError;

pub(crate) fn validate_state(state: AppState) -> Result<AppState, StateStoreError> {
    state.validate().map_err(StateStoreError::InvalidState)?;
    Ok(state)
}

impl From<StateValidationError> for StateStoreError {
    fn from(error: StateValidationError) -> Self {
        Self::InvalidState(error)
    }
}
