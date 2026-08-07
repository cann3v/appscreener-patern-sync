use reqwest::StatusCode;
use thiserror::Error;

const MAX_RESPONSE_BODY_CHARS: usize = 4_096;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{action}: HTTP transport error")]
    Transport {
        action: String,

        #[source]
        source: reqwest::Error,
    },

    #[error("{action}: appScreener returned HTTP {status}; response: {body}")]
    UnexpectedStatus {
        action: String,
        status: StatusCode,
        body: String,
    },

    #[error("{action}: failed to decode appScreener response")]
    InvalidResponse {
        action: String,

        #[source]
        source: reqwest::Error,
    },
}

pub fn limited_response_body(body: &str) -> String {
    let mut result: String = body.chars().take(MAX_RESPONSE_BODY_CHARS).collect();

    if body.chars().count() > MAX_RESPONSE_BODY_CHARS {
        result.push_str("…<truncated>");
    }

    if result.is_empty() {
        "<empty body>".to_owned()
    } else {
        result
    }
}
