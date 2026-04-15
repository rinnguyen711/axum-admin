use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("not found")]
    NotFound,

    #[error("validation error: {0:?}")]
    ValidationError(HashMap<String, String>),

    #[error("database error: {0}")]
    DatabaseError(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("error: {0}")]
    Custom(String),

    #[error("internal error: {0}")]
    Internal(String),
}
