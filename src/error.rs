use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    MissingField(String),
    Multipart(String),
    FileTooLarge {
        size: usize,
        max_size: usize,
    },
    InvalidFileType(String),
    Configuration(String),
    Database(String),
    #[from]
    ServerInit(std::io::Error),
    #[from]
    S3Upload(object_store::Error),
    #[from]
    PathParse(object_store::path::Error),
    #[from]
    JsonParsing(serde_json::Error),
    #[from]
    WebhookSend(reqwest::Error),
    #[from]
    Migration(Box<dyn std::error::Error + Send + Sync>),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match &self {
            Error::MissingField(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let error_message = match self {
            Error::MissingField(msg) => format!("Missing required field or filename: {}", msg),
            Error::Multipart(msg) => format!("Multipart processing error: {}", msg),
            Error::FileTooLarge { size, max_size } => {
                format!("File too large: {} bytes (max: {} bytes)", size, max_size)
            }
            Error::InvalidFileType(msg) => format!("Invalid file type: {}", msg),
            Error::Configuration(msg) => format!("Configuration error: {}", msg),
            Error::Database(msg) => format!("Database error: {}", msg),
            Error::S3Upload(msg) => format!("S3 Upload Error: {}", msg),
            Error::PathParse(msg) => format!("Invalid object path: {}", msg),
            Error::JsonParsing(msg) => format!("Json Parsing Error: {}", msg),
            Error::WebhookSend(msg) => format!("Webhook Send Error: {}", msg),
            Error::ServerInit(msg) => format!("Server Initialization Error: {}", msg),
            Error::Migration(msg) => format!("DB migration error: {}", msg),
        };
        println!("{:#?} status for {:#?}", status, error_message);
        (status, error_message).into_response()
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
