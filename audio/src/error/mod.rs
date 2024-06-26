
use std::fmt;
use serde_json::Error as SerdeJsonError;
use rusqlite::Error as RusqliteError;


#[derive(Debug)]
pub enum AppError {
    IoError(std::io::Error),
    PortAudioError(portaudio::Error),
    SerdeJsonError(SerdeJsonError),
    RusqliteError(RusqliteError),
    Other(String),  // For other types of errors
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter <'_>) -> fmt::Result {
        match self {
            AppError::IoError(e) => write!(f, "IO Error: {}", e),
            AppError::PortAudioError(e) => write!(f, "PortAudio Error: {}", e),
            AppError::Other(e) => write!(f, "Other Error: {}", e),
            AppError::RusqliteError(e) => write!(f, "Rusqlite Error: {}", e),
            AppError::SerdeJsonError(e) => write!(f, "Serde JSON Error: {}", e),
        }
    }
}
impl From<SerdeJsonError> for AppError {
    fn from(error: SerdeJsonError) -> Self {
        AppError::SerdeJsonError(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e)
    }
}
impl From<portaudio::Error> for AppError {
    fn from(e: portaudio::Error) -> Self {
        AppError::PortAudioError(e)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::RusqliteError(error)
    }
}