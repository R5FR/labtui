//! Smoke-test the real GitLab API path without the TUI.
//!
//! Usage (token from env):
//!   GITLAB_TOKEN=glpat-xxx cargo run -p asyncgitlab --example list_mrs -- <remote-url>
//!   GITLAB_TOKEN=glpat-xxx cargo run -p asyncgitlab --example list_mrs -- <host> <group/project>
//!
//! Examples:
//!   ... --example list_mrs -- https://gitlab.com/gitlab-org/gitlab.git
//!   ... --example list_mrs -- git@gitlab.com:gitlab-org/gitlab.git
//!   ... --example list_mrs -- gitlab.com gitlab-org/gitlab

use asyncgitlab::{
	runtime, GitLabClient, GitLabRemote, MergeRequestScope,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args: Vec<String> = std::env::args().skip(1).collect();

	let remote = match args.as_slice() {
		[url] => GitLabRemote::from_url(url)?,
		[host, path] => GitLabRemote {
			host: host.clone(),
			project_path: path.clone(),
		},
		_ => {
			eprintln!(
				"usage: list_mrs <remote-url> | list_mrs <host> <group/project>"
			);
			std::process::exit(2);
		}
	};

	println!(
		"→ host: {}\n→ project: {}\n→ api: {}\n",
		remote.host,
		remote.project_path,
		remote.api_base()
	);

	let client = GitLabClient::from_env(remote)?;
	let mrs = runtime::block_on(
		client.merge_requests(MergeRequestScope::Opened),
	)?;

	println!("{} open merge request(s):", mrs.len());
	for mr in mrs {
		let draft = if mr.draft { " [draft]" } else { "" };
		let author = mr
			.author
			.as_ref()
			.map_or_else(String::new, |a| format!(" @{}", a.username));
		println!(
			"  !{:<5} {:?}{draft} {} ({} → {}){author}",
			mr.iid,
			mr.state,
			mr.title,
			mr.source_branch,
			mr.target_branch,
		);
	}

	Ok(())
}
