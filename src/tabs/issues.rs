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
	has_token, store_token, AsyncActionJob, AsyncGitLabNotification,
	AsyncIssuesJob, GitLabAction, GitLabRemote, Issue, IssueScope,
	IssueState, StateEvent,
};
use crossterm::event::{Event, KeyCode};
use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph},
	Frame,
};

/// Loading state of the issue list.
enum LoadState {
	/// no GitLab remote could be detected for this repo
	NoRemote,
	/// a GitLab remote exists but no token is available yet
	NeedToken,
	/// request in flight, nothing loaded yet
	Loading,
	/// loaded issues (possibly empty)
	Loaded(Vec<Issue>),
	/// request failed
	Error(String),
}

pub struct IssuesTab {
	visible: bool,
	remote: Option<GitLabRemote>,
	state: LoadState,
	selection: usize,
	/// transient one-line feedback after a write action
	status_msg: Option<String>,
	async_issues: AsyncSingleJob<AsyncIssuesJob>,
	async_action: AsyncSingleJob<AsyncActionJob>,
	token_input: TextInputComponent,
	new_issue_input: TextInputComponent,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl IssuesTab {
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
			"paste a token with api scope, then press [Enter]",
			false,
		)
		.with_input_type(InputType::Password);

		let new_issue_input = TextInputComponent::new(
			env,
			"New issue",
			"issue title, then press [Enter]",
			false,
		);

		Self {
			visible: false,
			state,
			remote,
			selection: 0,
			status_msg: None,
			async_issues: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_action: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			token_input,
			new_issue_input,
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
			if !self.async_issues.is_pending()
				&& !self.async_action.is_pending()
			{
				if matches!(self.state, LoadState::NeedToken) {
					self.state = LoadState::Loading;
				}
				self.async_issues.spawn(AsyncIssuesJob::new(
					remote,
					IssueScope::Opened,
				));
			}
		} else {
			self.state = LoadState::NeedToken;
			self.show_token_prompt();
		}
	}

	/// Force a reload of the issue list.
	fn reload(&self) {
		if let Some(remote) = self.remote.clone() {
			if has_token(&remote.host)
				&& !self.async_issues.is_pending()
			{
				self.async_issues.spawn(AsyncIssuesJob::new(
					remote,
					IssueScope::Opened,
				));
			}
		}
	}

	fn show_token_prompt(&mut self) {
		if !self.token_input.is_visible() {
			self.token_input.clear();
			let _ = self.token_input.show();
		}
	}

	/// True while the user is entering text (used to block global quit).
	pub fn is_editing(&self) -> bool {
		self.token_input.is_visible()
			|| self.new_issue_input.is_visible()
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

	fn show_new_issue_prompt(&mut self) {
		if !self.new_issue_input.is_visible() {
			self.new_issue_input.clear();
			let _ = self.new_issue_input.show();
		}
	}

	/// Spawn a "create issue" action with the typed title.
	fn submit_new_issue(&mut self) {
		let title = self.new_issue_input.get_text().trim().to_string();
		self.new_issue_input.clear();
		self.new_issue_input.hide();
		if title.is_empty() {
			return;
		}
		self.spawn_action(GitLabAction::CreateIssue {
			title,
			description: None,
		});
	}

	/// Close the currently selected issue.
	fn close_selected(&mut self) {
		let Some(iid) = self.selected_issue().map(|i| i.iid) else {
			return;
		};
		self.spawn_action(GitLabAction::SetIssueState {
			iid,
			event: StateEvent::Close,
		});
	}

	fn spawn_action(&mut self, action: GitLabAction) {
		let Some(remote) = self.remote.clone() else {
			return;
		};
		if self.async_action.is_pending() {
			return;
		}
		self.status_msg = Some("working…".to_string());
		self.async_action
			.spawn(AsyncActionJob::new(remote, action));
	}

	/// handle a finished GitLab job
	pub fn update_gitlab(&mut self, ev: AsyncGitLabNotification) {
		match ev {
			AsyncGitLabNotification::Issues => {
				if let Some(job) = self.async_issues.take_last() {
					if let Some(result) = job.result() {
						self.state = match result {
							Ok(issues) => LoadState::Loaded(issues),
							Err(e) => LoadState::Error(e),
						};
						self.clamp_selection();
					}
				}
			}
			AsyncGitLabNotification::Action => {
				if let Some(job) = self.async_action.take_last() {
					if let Some(result) = job.result() {
						self.status_msg = Some(match result {
							Ok(msg) => msg,
							Err(e) => format!("error: {e}"),
						});
						// reflect the change in the list
						self.reload();
					}
				}
			}
			AsyncGitLabNotification::MergeRequests => {}
		}
	}

	pub fn any_work_pending(&self) -> bool {
		self.async_issues.is_pending()
			|| self.async_action.is_pending()
	}

	fn loaded(&self) -> Option<&[Issue]> {
		match &self.state {
			LoadState::Loaded(issues) => Some(issues),
			_ => None,
		}
	}

	fn selected_issue(&self) -> Option<&Issue> {
		self.loaded().and_then(|i| i.get(self.selection))
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
			|| "Issues".to_string(),
			|r| format!("Issues · {}", r.project_path),
		)
	}

	fn host(&self) -> &str {
		self.remote.as_ref().map_or("", |r| r.host.as_str())
	}

	fn render_list(
		&self,
		f: &mut Frame,
		rect: Rect,
		issues: &[Issue],
	) {
		// split off a one-line footer for transient action feedback
		let (list_area, footer) = self.status_msg.as_deref().map_or(
			(rect, None),
			|msg| {
				let chunks = Layout::default()
					.direction(Direction::Vertical)
					.constraints([
						Constraint::Min(1),
						Constraint::Length(1),
					])
					.split(rect);
				(chunks[0], Some((chunks[1], msg)))
			},
		);

		let items: Vec<ListItem> = issues
			.iter()
			.enumerate()
			.map(|(i, issue)| {
				let selected = i == self.selection;
				let marker = match issue.state {
					IssueState::Closed => "✗",
					_ => "●",
				};
				let author = issue
					.author
					.as_ref()
					.map_or_else(String::new, |a| {
						format!(" @{}", a.username)
					});
				let comments = if issue.user_notes_count > 0 {
					format!("  💬{}", issue.user_notes_count)
				} else {
					String::new()
				};
				let line = Line::from(vec![Span::styled(
					format!(
						"{marker} #{}  {}{author}{comments}",
						issue.iid, issue.title,
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
		f.render_widget(list, list_area);

		if let Some((rect, msg)) = footer {
			let p = Paragraph::new(msg)
				.style(self.theme.text(true, false));
			f.render_widget(p, rect);
		}
	}
}

impl DrawableComponent for IssuesTab {
	fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()> {
		match &self.state {
			LoadState::NoRemote => self.draw_message(
				f,
				rect,
				"No GitLab remote detected for this repository.",
			),
			LoadState::NeedToken => {
				if self.token_input.is_visible() {
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
						&strings::gitlab_token_help(
							self.host(),
							true,
						),
					);
				}
			}
			LoadState::Loading => {
				self.draw_message(f, rect, "Loading issues…");
			}
			LoadState::Error(e) => self.draw_message(
				f,
				rect,
				&format!(
					"Failed to load issues:\n{e}\n\nPress [Enter] to re-enter the token."
				),
			),
			LoadState::Loaded(issues) if issues.is_empty() => {
				self.draw_message(
					f,
					rect,
					"No open issues.\n\nPress [n] to create one.",
				);
			}
			LoadState::Loaded(issues) => {
				self.render_list(f, rect, issues);
			}
		}

		// new-issue input renders on top of the list when active
		if self.new_issue_input.is_visible() {
			self.new_issue_input.draw(f, rect)?;
		}

		Ok(())
	}
}

impl Component for IssuesTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				self.loaded().is_some_and(|i| !i.is_empty()),
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::issue_new(&self.key_config),
				true,
				self.loaded().is_some(),
			));
			out.push(CommandInfo::new(
				strings::commands::issue_close(&self.key_config),
				self.selected_issue().is_some(),
				self.loaded().is_some(),
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

		// while entering a new issue title, the input owns all keys
		if self.new_issue_input.is_visible() {
			if self.new_issue_input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}
			if let Event::Key(k) = ev {
				if key_match(k, self.key_config.keys.enter) {
					self.submit_new_issue();
				} else if key_match(
					k,
					self.key_config.keys.exit_popup,
				) {
					self.new_issue_input.hide();
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
			} else if matches!(k.code, KeyCode::Char('n'))
				&& self.loaded().is_some()
			{
				self.show_new_issue_prompt();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('c'))
				&& self.selected_issue().is_some()
			{
				self.close_selected();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('r'))
				&& self.loaded().is_some()
			{
				self.reload();
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
