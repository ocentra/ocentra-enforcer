// FAIL fixture for RESIL-IO-TIMEOUT.1: a bare external network call with
// no timeout/retry guard anywhere in the file.

async fn ping_upstream(url: &str) -> Result<String, reqwest::Error> {
    let response = reqwest::get(url).await?;
    response.text().await
}
