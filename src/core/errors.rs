use thiserror::Error;

#[derive(Debug)]
pub enum BadRequestType {
    ParseFailure,
    InvalidParam,
    ValidationFailure,
    Duplicate,
    Conflict,
    Prunable,
}

#[derive(Debug)]
pub enum UnavailableType {
    NetworkFailure,
    StorageFailure,
}

#[derive(Error, Debug)]
pub enum HubError {
    #[error("the request did not have valid authentication credentials")]
    Unauthenticated,
    #[error("the authenticated request did not have the authority to perform this action")]
    Unauthorized,
    #[error(
        "the request cannot be completed as constructed, do not retry (Type: {0:?}, Message: {1})"
    )]
    BadRequest(BadRequestType, String),
    #[error("the requested resource could not be found")]
    NotFound,
    #[error("the request could not be completed because the operation is not executable")]
    NotImplemented,
    #[error("the request could not be completed, it may or may not be safe to retry {0:?}")]
    Unavailable(UnavailableType),
    #[error("an unknown error was encountered")]
    Unknown,
}
