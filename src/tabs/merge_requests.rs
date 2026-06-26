use crate::{
	app::Environment,
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState, InputType, TextInputComponent,
	},
	keys::{key_match, SharedKeyConfig},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	asyncjob::AsyncSingleJob,
	sync::{get_default_remote, get_remote_url, RepoPathRef},
};
use asyncgitlab::{
	has_token, store_token, AsyncGitLabNotification,
	AsyncMergeRequestsJob, GitLabRemote, MergeRequest,
	MergeRequestScope, MergeRequestState,
};
use crossterm::event::Event;
use ratatui::{
	layout::{Alignment, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph},
	Frame,
};

/// Loading state of the merge request list.
enum LoadState {
	/// no GitLab remote could be detected for this repo
	NoRemote,
	/// a GitLab remote exists but no token is available yet
	NeedToken,
	/// request in flight, nothing loaded yet
	Loading,
	/// loaded merge requests (possibly empty)
	Loaded(Vec<MergeRequest>),
	/// request failed
	Error(String),
}

pub struct MergeRequestsTab {
	visible: bool,
	remote: Option<GitLabRemote>,
	state: LoadState,
	selection: usize,
	async_mrs: AsyncSingleJob<AsyncMergeRequestsJob>,
	token_input: TextInputComponent,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl MergeRequestsTab {
	pub fn new(env: &Environment) -> Self {
		let remote = detect_gitlab_remote(&env.repo);

		let state = match &remote {
			None => LoadState::NoRemote,
			Some(r) if has_token(&r.host) => LoadState::Loading,
			Some(_) => LoadState::NeedToken,
		};

		let token_input = TextInputComponent::new(
			env,
			"GitLab token",
			"paste a token with read_api scope, then press [Enter]",
			false,
		)
		.with_input_type(InputType::Password);

		Self {
			visible: false,
			state,
			remote,
			selection: 0,
			async_mrs: AsyncSingleJob::new(env.sender_gitlab.clone()),
			token_input,
			theme: env.theme.clone(),
			key_config: env.key_config.clone(),
		}
	}

	pub fn update(&mut self) {
		if self.is_visible() {
			self.ensure_load();
		}
	}

	/// Decide what to do based on remote + token availability: spawn a load,
	/// or surface the token prompt.
	fn ensure_load(&mut self) {
		let Some(remote) = self.remote.clone() else {
			return;
		};

		if has_token(&remote.host) {
			if self.token_input.is_visible() {
				self.token_input.hide();
			}
			if !self.async_mrs.is_pending() {
				if matches!(self.state, LoadState::NeedToken) {
					self.state = LoadState::Loading;
				}
				self.async_mrs.spawn(AsyncMergeRequestsJob::new(
					remote,
					MergeRequestScope::Opened,
				));
			}
		} else {
			self.state = LoadState::NeedToken;
			self.show_token_prompt();
		}
	}

	fn show_token_prompt(&mut self) {
		if !self.token_input.is_visible() {
			self.token_input.clear();
			let _ = self.token_input.show();
		}
	}

	/// True while the user is entering a token (used to block global quit).
	pub fn is_editing(&self) -> bool {
		self.token_input.is_visible()
	}

	/// Persist the typed token to the OS keyring, then start loading.
	fn submit_token(&mut self) {
		let token = self.token_input.get_text().trim().to_string();
		if token.is_empty() {
			return;
		}

		let Some(host) =
			self.remote.as_ref().map(|r| r.host.clone())
		else {
			return;
		};

		match store_token(&host, &token) {
			Ok(()) => {
				self.token_input.clear();
				self.token_input.hide();
				self.state = LoadState::Loading;
				self.ensure_load();
			}
			Err(e) => {
				self.token_input.hide();
				self.state = LoadState::Error(format!(
					"could not store token in keyring: {e}"
				));
			}
		}
	}

	/// handle a finished GitLab job
	pub fn update_gitlab(&mut self, ev: AsyncGitLabNotification) {
		match ev {
			AsyncGitLabNotification::MergeRequests => {
				if let Some(job) = self.async_mrs.take_last() {
					if let Some(result) = job.result() {
						self.state = match result {
							Ok(mrs) => LoadState::Loaded(mrs),
							Err(e) => LoadState::Error(e),
						};
						self.clamp_selection();
					}
				}
			}
		}
	}

	pub fn any_work_pending(&self) -> bool {
		self.async_mrs.is_pending()
	}

	fn loaded(&self) -> Option<&[MergeRequest]> {
		match &self.state {
			LoadState::Loaded(mrs) => Some(mrs),
			_ => None,
		}
	}

	fn clamp_selection(&mut self) {
		let len = self.loaded().map_or(0, <[_]>::len);
		if len == 0 {
			self.selection = 0;
		} else if self.selection >= len {
			self.selection = len - 1;
		}
	}

	fn move_selection(&mut self, down: bool) {
		let len = self.loaded().map_or(0, <[_]>::len);
		if len == 0 {
			return;
		}
		if down {
			self.selection = (self.selection + 1) % len;
		} else {
			self.selection =
				self.selection.checked_sub(1).unwrap_or(len - 1);
		}
	}

	fn draw_message(&self, f: &mut Frame, rect: Rect, msg: &str) {
		let block = Block::default()
			.borders(Borders::ALL)
			.title(self.title());
		let p = Paragraph::new(msg)
			.block(block)
			.alignment(Alignment::Center)
			.style(self.theme.text(true, false));
		f.render_widget(p, rect);
	}

	fn title(&self) -> String {
		self.remote.as_ref().map_or_else(
			|| "Merge Requests".to_string(),
			|r| format!("Merge Requests · {}", r.project_path),
		)
	}

	fn host(&self) -> &str {
		self.remote.as_ref().map_or("", |r| r.host.as_str())
	}

	fn render_list(&self, f: &mut Frame, rect: Rect, mrs: &[MergeRequest]) {
		let items: Vec<ListItem> = mrs
			.iter()
			.enumerate()
			.map(|(i, mr)| {
				let selected = i == self.selection;
				let marker = match mr.state {
					MergeRequestState::Merged => "✓",
					MergeRequestState::Closed => "✗",
					_ if mr.draft => "◐",
					_ => "●",
				};
				let author = mr
					.author
					.as_ref()
					.map_or_else(String::new, |a| {
						format!(" @{}", a.username)
					});
				let line = Line::from(vec![Span::styled(
					format!(
						"{marker} !{}  {}  ({} → {}){author}",
						mr.iid,
						mr.title,
						mr.source_branch,
						mr.target_branch,
					),
					self.theme.text(true, selected),
				)]);
				ListItem::new(line)
			})
			.collect();

		let list = List::new(items).block(
			Block::default()
				.borders(Borders::ALL)
				.title(self.title()),
		);
		f.render_widget(list, rect);
	}
}

impl DrawableComponent for MergeRequestsTab {
	fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()> {
		match &self.state {
			LoadState::NoRemote => self.draw_message(
				f,
				rect,
				"No GitLab remote detected for this repository.",
			),
			LoadState::NeedToken => {
				if self.token_input.is_visible() {
					// underlying message + centered input popup on top
					self.draw_message(
						f,
						rect,
						&format!(
							"A GitLab token is required for {}.",
							self.host()
						),
					);
					self.token_input.draw(f, rect)?;
				} else {
					self.draw_message(
						f,
						rect,
						&format!(
							"A GitLab token is required for {}.\n\nPress [Enter] to set it.",
							self.host()
						),
					);
				}
			}
			LoadState::Loading => {
				self.draw_message(f, rect, "Loading merge requests…");
			}
			LoadState::Error(e) => self.draw_message(
				f,
				rect,
				&format!(
					"Failed to load merge requests:\n{e}\n\nPress [Enter] to re-enter the token."
				),
			),
			LoadState::Loaded(mrs) if mrs.is_empty() => {
				self.draw_message(f, rect, "No open merge requests.");
			}
			LoadState::Loaded(mrs) => self.render_list(f, rect, mrs),
		}

		Ok(())
	}
}

impl Component for MergeRequestsTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				self.loaded().is_some_and(|m| !m.is_empty()),
				true,
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if !self.is_visible() {
			return Ok(EventState::NotConsumed);
		}

		// while entering a token, the input owns all keys
		if self.token_input.is_visible() {
			if self.token_input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}
			if let Event::Key(k) = ev {
				if key_match(k, self.key_config.keys.enter) {
					self.submit_token();
				} else if key_match(
					k,
					self.key_config.keys.exit_popup,
				) {
					self.token_input.hide();
				}
			}
			return Ok(EventState::Consumed);
		}

		if let Event::Key(k) = ev {
			if key_match(k, self.key_config.keys.move_down) {
				self.move_selection(true);
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.move_up) {
				self.move_selection(false);
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.enter)
				&& matches!(
					self.state,
					LoadState::NeedToken | LoadState::Error(_)
				) {
				self.show_token_prompt();
				return Ok(EventState::Consumed);
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		self.ensure_load();
		Ok(())
	}
}

/// Inspect the default remote and parse it into a GitLab project, if any.
fn detect_gitlab_remote(
	repo: &RepoPathRef,
) -> Option<GitLabRemote> {
	let repo = repo.borrow();
	let remote_name = get_default_remote(&repo).ok()?;
	let url = get_remote_url(&repo, &remote_name).ok()??;
	GitLabRemote::from_url(&url).ok()
}
