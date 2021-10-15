use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("http error")]
    ReqwestError(#[from] reqwest::Error),
}

pub type AppResult<T> = Result<T, AppError>;
