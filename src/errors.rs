use actix_web::{http::StatusCode, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("http error")]
    ReqwestError(#[from] reqwest::Error),

    #[error("internal server error")]
    InternalError,

    #[error("authentication failed")]
    InvalidUsernamePassword,

    #[error("invalid token")]
    InvalidToken,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::ReqwestError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidUsernamePassword => StatusCode::BAD_REQUEST,
            AppError::InvalidToken => StatusCode::UNAUTHORIZED,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
