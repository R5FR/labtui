//! Open a URL in the user's default browser, cross-platform, with no extra
//! dependency: we just spawn the platform's URL handler.

use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

/// Launch `url` in the default browser. Returns once the handler is spawned;
/// it does not wait for the browser to exit.
pub fn open_in_browser(url: &str) -> Result<()> {
	#[cfg(target_os = "macos")]
	let (cmd, args): (&str, Vec<&str>) = ("open", vec![url]);
	#[cfg(target_os = "windows")]
	let (cmd, args): (&str, Vec<&str>) =
		("cmd", vec!["/C", "start", "", url]);
	#[cfg(all(unix, not(target_os = "macos")))]
	let (cmd, args): (&str, Vec<&str>) = ("xdg-open", vec![url]);

	Command::new(cmd)
		.args(&args)
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.map_err(|e| {
			anyhow!("failed to open browser (`{cmd}`): {e}")
		})?;
	Ok(())
}
