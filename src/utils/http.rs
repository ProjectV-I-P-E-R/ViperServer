use reqwest::{Client as HttpClient, header};
use std::time::Duration;
use anyhow::{Error, Result};
use crate::utils::config::AppConfig;

pub fn create_http_client(config: &AppConfig) -> Result<HttpClient, Error> {
    let user_agent = format!("VIPER-Engine/{}", config.build_version);
    let mut headers = header::HeaderMap::new();

    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::try_from(user_agent)?
    );

    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json")
    );

    let client = HttpClient::builder()
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(60))
        .build()?;

    Ok(client)
}