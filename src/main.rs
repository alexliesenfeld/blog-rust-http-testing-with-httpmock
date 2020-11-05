use custom_error::custom_error;
use isahc::prelude::Request;
use isahc::{RequestExt, ResponseExt};
use serde_json::{json, Value};

use crate::GithubError::{MissingValueError, UnexpectedResponseCodeError};

custom_error! {pub GithubError
    HttpError{source: isahc::http::Error} = "HTTP error",
    HttpClientError{source: isahc::Error} = "HTTP client error",
    ParserError{source: serde_json::Error} = "JSON parser error",
    UnexpectedResponseCodeError{code: u16} = "Unexpected HTTP response code: {code}",
    MissingValueError{field: &'static str} = "Missing field in HTTP response: {field}",
}

pub struct GithubClient {
    base_url: String,
    token: String,
}

impl GithubClient {
    pub fn new(token: &str, base_url: &str) -> GithubClient {
        GithubClient { base_url: base_url.into(), token: token.into() }
    }

    pub fn create_repo(&self, name: &str) -> Result<String, GithubError> {
        let mut response = Request::post(format!("{}/user/repos", self.base_url))
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .body(json!({ "name": name, "private": true }).to_string())?
            .send()?;

        if response.status() != 201 {
            return Err(UnexpectedResponseCodeError {
                code: response.status().as_u16(),
            });
        }

        let json_body: Value = response.json()?;
        return match json_body["html_url"].as_str() {
            Some(url) => Ok(url.into()),
            None => Err(MissingValueError { field: "html_url" }),
        };
    }
}

fn main() {
    let github = GithubClient::new("<github-token>", "https://api.github.com");
    let url = github.create_repo("myRepo").expect("Cannot create repo");
    println!("Repo URL: {}", url);
}

#[cfg(test)]
mod tests {
    use httpmock::MockServer;
    use serde_json::json;

    use crate::GithubClient;

    #[test]
    fn create_repo_success_test() {
        let _ = env_logger::try_init();

        // Arrange
        let mock_server = MockServer::start();

        let mock = mock_server.mock(|when, then| {
            when.method("POST")
                .path("/user/repos")
                .header("Authorization", "token TOKEN")
                .header("Content-Type", "application/json");
            then.status(201)
                .json_body(json!({ "html_url": "http://example.com" }));
        });

        let github_client = GithubClient::new("TOKEN".into(), &mock_server.base_url());

        // Act
        let result = github_client.create_repo("myRepo");

        // Assert
        mock.assert();
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), "http://example.com");
    }
}
