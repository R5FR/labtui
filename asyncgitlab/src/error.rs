//! error types for the GitLab API layer

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
	#[error("remote `{0}` does not look like a GitLab repository URL")]
	UnsupportedRemote(String),

	#[error("no GitLab token found (set GITLAB_TOKEN or configure one)")]
	MissingToken,

	#[error("credential store error: {0}")]
	Keyring(String),

	#[error("http error: {0}")]
	Http(#[from] reqwest::Error),

	#[error("gitlab api returned status {status}: {body}")]
	Api { status: u16, body: String },

	#[error("url parse error: {0}")]
	Url(#[from] url::ParseError),

	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
