//! Token resolution for GitLab authentication.
//!
//! Lookup order for a host's token:
//!   1. host-specific env var `GITLAB_TOKEN_<HOST>` (override, never stored)
//!   2. generic env var `GITLAB_TOKEN` (override, never stored)
//!   3. the OS credential store (gnome-keyring / KWallet / Keychain), keyed by host
//!
//! Tokens entered interactively are persisted with [`store_token`], which writes
//! to the OS keyring only — never to a plaintext file.

use crate::error::{Error, Result};

/// Environment variable holding a GitLab personal/project access token.
pub const TOKEN_ENV: &str = "GITLAB_TOKEN";

/// Service name under which tokens are filed in the OS credential store.
const KEYRING_SERVICE: &str = "labtui";

/// Per-host override, e.g. `GITLAB_TOKEN_GITLAB_EXAMPLE_COM`.
fn host_env_var(host: &str) -> String {
	let suffix: String = host
		.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
		.collect();
	format!("{TOKEN_ENV}_{suffix}")
}

/// A token supplied through the environment, if any (host-specific first).
fn env_token(host: &str) -> Option<String> {
	for var in [host_env_var(host), TOKEN_ENV.to_string()] {
		if let Ok(t) = std::env::var(&var) {
			if !t.is_empty() {
				return Some(t);
			}
		}
	}
	None
}

/// Build the credential-store entry for a host.
fn keyring_entry(host: &str) -> Result<keyring::Entry> {
	keyring::Entry::new(KEYRING_SERVICE, host)
		.map_err(|e| Error::Keyring(e.to_string()))
}

/// Fetch a token from the OS credential store, `Ok(None)` if none is stored.
pub fn keyring_token(host: &str) -> Result<Option<String>> {
	match keyring_entry(host)?.get_password() {
		Ok(t) => Ok(Some(t)),
		Err(keyring::Error::NoEntry) => Ok(None),
		Err(e) => Err(Error::Keyring(e.to_string())),
	}
}

/// Persist a token for `host` in the OS credential store.
pub fn store_token(host: &str, token: &str) -> Result<()> {
	keyring_entry(host)?
		.set_password(token)
		.map_err(|e| Error::Keyring(e.to_string()))
}

/// Remove a stored token for `host` (no error if there was none).
pub fn delete_token(host: &str) -> Result<()> {
	match keyring_entry(host)?.delete_credential() {
		Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
		Err(e) => Err(Error::Keyring(e.to_string())),
	}
}

/// True if a token is available for `host` (env or credential store) without
/// surfacing keyring errors — handy for the UI to decide whether to prompt.
pub fn has_token(host: &str) -> bool {
	if env_token(host).is_some() {
		return true;
	}
	matches!(keyring_token(host), Ok(Some(_)))
}

/// Resolve a token for `host`: env override first, then the OS credential store.
pub fn resolve_token(host: &str) -> Result<String> {
	if let Some(t) = env_token(host) {
		return Ok(t);
	}
	if let Some(t) = keyring_token(host)? {
		return Ok(t);
	}
	Err(Error::MissingToken)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn host_var_naming() {
		assert_eq!(host_env_var("gitlab.com"), "GITLAB_TOKEN_GITLAB_COM");
		assert_eq!(
			host_env_var("gitlab.example.com"),
			"GITLAB_TOKEN_GITLAB_EXAMPLE_COM"
		);
	}
}
