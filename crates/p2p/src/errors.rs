use thiserror::Error;

#[derive(Debug)]
pub enum BadRequestType {
    Generic,
    ParseFailure,
    InvalidParam,
    ValidationFailure,
    Duplicate,
    Conflict,
    Prunable,
}

#[derive(Debug)]
pub enum UnavailableType {
    Generic,
    NetworkFailure,
    StorageFailure,
}

#[derive(Error, Debug)]
pub enum P2pError {
    #[error("the request did not have valid authentication credentials {0:#?}")]
    Unauthenticated(String),
    #[error("the authenticated request did not have the authority to perform this action {0:#?}")]
    Unauthorized(String),
    #[error("the request cannot be completed as constructed, do not retry (Type: {0:?}, Message: {1:#?})")]
    BadRequest(BadRequestType, String),
    #[error("the requested resource could not be found {0:#?}")]
    NotFound(String),
    #[error("the request could not be completed because the operation is not executable {0:#?}")]
    NotImplemented(String),
    #[error("the request could not be completed, it may or may not be safe to retry {0:?} {1:#?}")]
    Unavailable(UnavailableType, String),
    #[error("an unknown error was encountered {0:#?}")]
    Unknown(String),
}
