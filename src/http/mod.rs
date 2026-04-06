use std::time::Duration;
use reqwest::{Client, ClientBuilder};

pub fn default_client(user_agent: &str, timeout_secs: u64) -> anyhow::Result<Client> {
    let client = ClientBuilder::new()
        .user_agent(user_agent)
        .gzip(true)
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    Ok(client)
}