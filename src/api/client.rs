use std::time::Duration;

use anyhow::{Context, Result, ensure};
use reqwest::blocking::{Client, Response};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, instrument};

use crate::api::dto::{PatternDto, PatternWrite};
use crate::error::{ApiError, limited_response_body};

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str, token: &str, timeout: Duration) -> Result<Self> {
        let base_url = normalize_base_url(base_url)?;

        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .context("JWT contains characters invalid in an HTTP header")?;

        // reqwest и middleware не должны выводить этот заголовок.
        authorization.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self { client, base_url })
    }

    #[instrument(skip(self), fields(rule_id = %rule_id))]
    pub fn verify_custom_rule(&self, rule_id: &str) -> Result<()> {
        let endpoint = format!("/rules/custom/{rule_id}/info");

        debug!(
            method = "GET",
            endpoint = "/rules/custom/{id}/info",
            "sending appScreener request"
        );

        let response = self
            .client
            .get(self.url(&endpoint))
            .send()
            .map_err(|source| ApiError::Transport {
                action: "verify custom rule".to_owned(),
                source,
            })?;

        let _: Value = decode_json(response, "verify custom rule")?;

        Ok(())
    }

    #[instrument(skip(self), fields(rule_id = %rule_id))]
    pub fn get_patterns(&self, rule_id: &str) -> Result<Vec<PatternDto>> {
        let endpoint = format!("/rules/custom/{rule_id}/patterns");

        debug!(
            method = "GET",
            endpoint = "/rules/custom/{id}/patterns",
            "sending appScreener request"
        );

        let response = self
            .client
            .get(self.url(&endpoint))
            .send()
            .map_err(|source| ApiError::Transport {
                action: "get rule patterns".to_owned(),
                source,
            })?;

        decode_json(response, "get rule patterns").map_err(Into::into)
    }

    #[instrument(
        skip(self, pattern),
        fields(pattern_name = %pattern.name)
    )]
    pub fn create_pattern(&self, pattern: &PatternWrite) -> Result<PatternDto> {
        debug!(
            method = "POST",
            endpoint = "/patterns/pattern",
            "sending appScreener request"
        );

        let response = self
            .client
            .post(self.url("/patterns/pattern"))
            .json(pattern)
            .send()
            .map_err(|source| ApiError::Transport {
                action: format!("create pattern {:?}", pattern.name),
                source,
            })?;

        decode_json(response, &format!("create pattern {:?}", pattern.name)).map_err(Into::into)
    }

    #[instrument(
        skip(self, pattern),
        fields(
            pattern_name = %pattern.name,
            pattern_uuid = ?pattern.uuid
        )
    )]
    pub fn update_pattern(&self, pattern: &PatternWrite) -> Result<PatternDto> {
        debug!(
            method = "PUT",
            endpoint = "/patterns/pattern",
            "sending appScreener request"
        );

        let response = self
            .client
            .put(self.url("/patterns/pattern"))
            .json(pattern)
            .send()
            .map_err(|source| ApiError::Transport {
                action: format!("update pattern {:?}", pattern.name),
                source,
            })?;

        decode_json(response, &format!("update pattern {:?}", pattern.name)).map_err(Into::into)
    }

    #[instrument(
        skip(self),
        fields(pattern_name = %name)
    )]
    pub fn delete_pattern(&self, uuid: &str, name: &str) -> Result<()> {
        debug!(
            method = "DELETE",
            endpoint = "/patterns/pattern",
            "sending appScreener request"
        );

        let response = self
            .client
            .delete(self.url("/patterns/pattern"))
            .query(&[("uuid", uuid)])
            .send()
            .map_err(|source| ApiError::Transport {
                action: format!("delete pattern {name:?}"),
                source,
            })?;

        checked_response(response, &format!("delete pattern {name:?}"))?;

        Ok(())
    }

    fn url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint)
    }
}

fn normalize_base_url(value: &str) -> Result<String> {
    let value = value.trim().trim_end_matches('/');

    ensure!(!value.is_empty(), "--base-url cannot be empty");

    let parsed = Url::parse(value).context("--base-url is not a valid URL")?;

    ensure!(
        matches!(parsed.scheme(), "http" | "https"),
        "--base-url must use http or https"
    );

    ensure!(
        parsed.host_str().is_some(),
        "--base-url must contain a host"
    );

    ensure!(
        parsed.query().is_none(),
        "--base-url must not contain query parameters"
    );

    ensure!(
        parsed.fragment().is_none(),
        "--base-url must not contain a fragment"
    );

    let base = value.to_owned();

    if parsed.path().contains("/api/") {
        return Ok(base);
    }

    if base.ends_with("/app") {
        Ok(format!("{base}/api/v1"))
    } else {
        Ok(format!("{base}/app/api/v1"))
    }
}

fn checked_response(response: Response, action: &str) -> Result<Response, ApiError> {
    let status = response.status();

    if status.is_success() {
        return Ok(response);
    }

    let body = response
        .text()
        .map(|body| limited_response_body(&body))
        .unwrap_or_else(|_| "<unreadable body>".to_owned());

    Err(ApiError::UnexpectedStatus {
        action: action.to_owned(),
        status,
        body: describe_status(status, body),
    })
}

fn decode_json<T>(response: Response, action: &str) -> Result<T, ApiError>
where
    T: DeserializeOwned,
{
    let response = checked_response(response, action)?;

    response
        .json::<T>()
        .map_err(|source| ApiError::InvalidResponse {
            action: action.to_owned(),
            source,
        })
}

fn describe_status(status: StatusCode, body: String) -> String {
    match status {
        StatusCode::UNAUTHORIZED => {
            format!("{body}; check APPSCREENER_TOKEN")
        }

        StatusCode::FORBIDDEN => {
            format!("{body}; token has no permission to modify this rule")
        }

        _ => body,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_base_url;

    #[test]
    fn appends_default_api_path() {
        let result = normalize_base_url("http://appscreener.local").unwrap();

        assert_eq!(result, "http://appscreener.local/app/api/v1");
    }

    #[test]
    fn preserves_explicit_api_path() {
        let result = normalize_base_url("https://appscreener.local/app/api/v1/").unwrap();

        assert_eq!(result, "https://appscreener.local/app/api/v1");
    }

    #[test]
    fn rejects_query_parameters() {
        assert!(normalize_base_url("https://appscreener.local?token=secret").is_err());
    }
}
