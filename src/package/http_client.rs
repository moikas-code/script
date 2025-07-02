/// HTTP client implementation for registry operations
use super::{PackageError, PackageResult};
use reqwest::blocking::{Client, ClientBuilder, Response};
use std::collections::HashMap;
use std::time::Duration;

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> PackageResult<Self> {
        Self::with_timeout(Duration::from_secs(30))
    }

    pub fn with_timeout(timeout: Duration) -> PackageResult<Self> {
        let client = ClientBuilder::new()
            .timeout(timeout)
            .user_agent(format!("manuscript/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| PackageError::Registry(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client })
    }

    pub fn get(&self, url: &str) -> PackageResult<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| PackageError::Registry(format!("GET request failed: {}", e)))?;

        self.handle_response(response)
    }

    pub fn get_with_headers(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> PackageResult<Vec<u8>> {
        let mut request = self.client.get(url);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .map_err(|e| PackageError::Registry(format!("GET request failed: {}", e)))?;

        self.handle_response(response)
    }

    pub fn post(&self, url: &str, body: Vec<u8>) -> PackageResult<Vec<u8>> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map_err(|e| PackageError::Registry(format!("POST request failed: {}", e)))?;

        self.handle_response(response)
    }

    pub fn post_with_auth(
        &self,
        url: &str,
        body: Vec<u8>,
        auth_token: &str,
    ) -> PackageResult<Vec<u8>> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", auth_token))
            .body(body)
            .send()
            .map_err(|e| PackageError::Registry(format!("POST request failed: {}", e)))?;

        self.handle_response(response)
    }

    pub fn head(&self, url: &str) -> PackageResult<bool> {
        let response = self
            .client
            .head(url)
            .send()
            .map_err(|e| PackageError::Registry(format!("HEAD request failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    pub fn download_with_progress<F>(
        &self,
        url: &str,
        mut progress_callback: F,
    ) -> PackageResult<Vec<u8>>
    where
        F: FnMut(u64, u64),
    {
        let mut response = self
            .client
            .get(url)
            .send()
            .map_err(|e| PackageError::Registry(format!("Download request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(PackageError::Registry(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut buffer = Vec::new();
        let mut downloaded = 0u64;

        use std::io::Read;
        let mut chunk = [0u8; 8192];

        loop {
            match response.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    downloaded += n as u64;
                    progress_callback(downloaded, total_size);
                }
                Err(e) => {
                    return Err(PackageError::Registry(format!(
                        "Download read error: {}",
                        e
                    )));
                }
            }
        }

        Ok(buffer)
    }

    fn handle_response(&self, response: Response) -> PackageResult<Vec<u8>> {
        if response.status().is_success() {
            response
                .bytes()
                .map(|b| b.to_vec())
                .map_err(|e| PackageError::Registry(format!("Failed to read response body: {}", e)))
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(PackageError::Registry(format!(
                "Request failed with status {}: {}",
                status, error_text
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_creation() {
        let client = HttpClient::new();
        assert!(client.is_ok());

        let client_with_timeout = HttpClient::with_timeout(Duration::from_secs(60));
        assert!(client_with_timeout.is_ok());
    }
}
