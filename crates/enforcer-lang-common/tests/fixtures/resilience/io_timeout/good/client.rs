// PASS fixture for RESIL-IO-TIMEOUT.1: the external call is wrapped in an
// explicit timeout guard.

async fn ping_upstream(url: &str) -> Result<String, reqwest::Error> {
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), reqwest::get(url))
        .await
        .map_err(|_| reqwest::Error::from(std::io::Error::from(std::io::ErrorKind::TimedOut)))??;
    response.text().await
}
