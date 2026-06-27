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
	AsyncPipelineJobsJob, AsyncPipelinesJob, AsyncTraceJob, CiStatus,
	GitLabAction, GitLabRemote, Job, Pipeline,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
	Frame,
};

enum Load<T> {
	Loading,
	Loaded(T),
	Error(String),
}

pub struct PipelinesTab {
	visible: bool,
	remote: Option<GitLabRemote>,
	pipelines: Load<Vec<Pipeline>>,
	pl_selection: usize,
	/// jobs of the opened pipeline, if drilled in
	jobs: Option<Load<Vec<Job>>>,
	jobs_pipeline_id: Option<u64>,
	job_selection: usize,
	/// trace of the opened job, if drilled in
	trace: Option<Load<String>>,
	trace_job_id: Option<u64>,
	trace_scroll: u16,
	status_msg: Option<String>,
	token_error: Option<String>,
	async_pipelines: AsyncSingleJob<AsyncPipelinesJob>,
	async_jobs: AsyncSingleJob<AsyncPipelineJobsJob>,
	async_trace: AsyncSingleJob<AsyncTraceJob>,
	async_action: AsyncSingleJob<AsyncActionJob>,
	token_input: TextInputComponent,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl PipelinesTab {
	pub fn new(env: &Environment) -> Self {
		let remote = detect_gitlab_remote(&env.repo);

		let token_input = TextInputComponent::new(
			env,
			"GitLab token",
			"paste a token with api scope, then press [Enter]",
			false,
		)
		.with_input_type(InputType::Password);

		Self {
			visible: false,
			remote,
			pipelines: Load::Loading,
			pl_selection: 0,
			jobs: None,
			jobs_pipeline_id: None,
			job_selection: 0,
			trace: None,
			trace_job_id: None,
			trace_scroll: 0,
			status_msg: None,
			token_error: None,
			async_pipelines: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_jobs: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_trace: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_action: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
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

	fn token_available(&self) -> bool {
		self.remote
			.as_ref()
			.is_some_and(|r| has_token(&r.host))
	}

	fn ensure_load(&mut self) {
		if !self.token_available() {
			return;
		}
		if self.token_input.is_visible() {
			self.token_input.hide();
		}
		let Some(remote) = self.remote.clone() else {
			return;
		};
		if self.async_action.is_pending() {
			return;
		}
		if matches!(self.pipelines, Load::Loading)
			&& !self.async_pipelines.is_pending()
		{
			self.async_pipelines
				.spawn(AsyncPipelinesJob::new(remote));
		}
	}

	fn reload(&mut self) {
		match self.level() {
			Level::Trace => self.reload_trace(),
			Level::Jobs => self.reload_jobs(),
			Level::Pipelines => {
				self.pipelines = Load::Loading;
				self.ensure_load();
			}
		}
	}

	fn show_token_prompt(&mut self) {
		if !self.token_input.is_visible() {
			self.token_input.clear();
			let _ = self.token_input.show();
		}
	}

	pub fn is_editing(&self) -> bool {
		self.token_input.is_visible()
	}

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
				self.token_error = None;
				self.pipelines = Load::Loading;
				self.ensure_load();
			}
			Err(e) => {
				self.token_input.hide();
				self.token_error = Some(format!(
					"could not store token in keyring: {e}"
				));
			}
		}
	}

	const fn level(&self) -> Level {
		if self.trace.is_some() {
			Level::Trace
		} else if self.jobs.is_some() {
			Level::Jobs
		} else {
			Level::Pipelines
		}
	}

	fn pipelines_slice(&self) -> Option<&[Pipeline]> {
		match &self.pipelines {
			Load::Loaded(p) => Some(p),
			_ => None,
		}
	}

	fn jobs_slice(&self) -> Option<&[Job]> {
		match &self.jobs {
			Some(Load::Loaded(j)) => Some(j),
			_ => None,
		}
	}

	fn selected_pipeline(&self) -> Option<&Pipeline> {
		self.pipelines_slice().and_then(|p| p.get(self.pl_selection))
	}

	fn selected_job(&self) -> Option<&Job> {
		self.jobs_slice().and_then(|j| j.get(self.job_selection))
	}

	fn open_jobs(&mut self) {
		let (Some(id), Some(remote)) = (
			self.selected_pipeline().map(|p| p.id),
			self.remote.clone(),
		) else {
			return;
		};
		self.jobs_pipeline_id = Some(id);
		self.job_selection = 0;
		self.jobs = Some(Load::Loading);
		self.async_jobs
			.spawn(AsyncPipelineJobsJob::new(remote, id));
	}

	fn reload_jobs(&mut self) {
		let (Some(id), Some(remote)) =
			(self.jobs_pipeline_id, self.remote.clone())
		else {
			return;
		};
		self.jobs = Some(Load::Loading);
		self.async_jobs
			.spawn(AsyncPipelineJobsJob::new(remote, id));
	}

	fn open_trace(&mut self) {
		let (Some(id), Some(remote)) =
			(self.selected_job().map(|j| j.id), self.remote.clone())
		else {
			return;
		};
		self.trace_job_id = Some(id);
		self.trace_scroll = 0;
		self.trace = Some(Load::Loading);
		self.async_trace.spawn(AsyncTraceJob::new(remote, id));
	}

	fn reload_trace(&mut self) {
		let (Some(id), Some(remote)) =
			(self.trace_job_id, self.remote.clone())
		else {
			return;
		};
		self.trace = Some(Load::Loading);
		self.async_trace.spawn(AsyncTraceJob::new(remote, id));
	}

	fn go_back(&mut self) {
		match self.level() {
			Level::Trace => {
				self.trace = None;
				self.trace_job_id = None;
				self.trace_scroll = 0;
			}
			Level::Jobs => {
				self.jobs = None;
				self.jobs_pipeline_id = None;
			}
			Level::Pipelines => {}
		}
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

	fn retry_selected(&mut self) {
		match self.level() {
			Level::Pipelines => {
				if let Some(id) = self.selected_pipeline().map(|p| p.id)
				{
					self.spawn_action(GitLabAction::RetryPipeline {
						id,
					});
				}
			}
			Level::Jobs => {
				if let Some(id) = self.selected_job().map(|j| j.id) {
					self.spawn_action(GitLabAction::RetryJob { id });
				}
			}
			Level::Trace => {}
		}
	}

	fn cancel_selected(&mut self) {
		match self.level() {
			Level::Pipelines => {
				if let Some(id) = self.selected_pipeline().map(|p| p.id)
				{
					self.spawn_action(
						GitLabAction::CancelPipeline { id },
					);
				}
			}
			Level::Jobs => {
				if let Some(id) = self.selected_job().map(|j| j.id) {
					self.spawn_action(GitLabAction::CancelJob {
						id,
					});
				}
			}
			Level::Trace => {}
		}
	}

	fn open_in_browser(&mut self) {
		let url = match self.level() {
			Level::Pipelines => {
				self.selected_pipeline().map(|p| p.web_url.clone())
			}
			Level::Jobs => {
				self.selected_job().map(|j| j.web_url.clone())
			}
			Level::Trace => None,
		};
		let Some(url) = url.filter(|u| !u.is_empty()) else {
			return;
		};
		if let Err(e) = crate::open_browser::open_in_browser(&url) {
			self.status_msg = Some(format!("error: {e}"));
		}
	}

	pub fn update_gitlab(&mut self, ev: AsyncGitLabNotification) {
		match ev {
			AsyncGitLabNotification::Pipelines => {
				if let Some(job) = self.async_pipelines.take_last() {
					if let Some(result) = job.result() {
						self.pipelines = match result {
							Ok(p) => Load::Loaded(p),
							Err(e) => Load::Error(e),
						};
						self.clamp_pl();
					}
				}
			}
			AsyncGitLabNotification::PipelineJobs => {
				if let Some(job) = self.async_jobs.take_last() {
					if let Some(result) = job.result() {
						self.jobs = Some(match result {
							Ok(j) => Load::Loaded(j),
							Err(e) => Load::Error(e),
						});
						self.clamp_job();
					}
				}
			}
			AsyncGitLabNotification::JobTrace => {
				if let Some(job) = self.async_trace.take_last() {
					if let Some(result) = job.result() {
						self.trace = Some(match result {
							Ok(t) => Load::Loaded(t),
							Err(e) => Load::Error(e),
						});
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
						self.reload();
					}
				}
			}
			_ => {}
		}
	}

	pub fn any_work_pending(&self) -> bool {
		self.async_pipelines.is_pending()
			|| self.async_jobs.is_pending()
			|| self.async_trace.is_pending()
			|| self.async_action.is_pending()
	}

	fn clamp_pl(&mut self) {
		let len = self.pipelines_slice().map_or(0, <[_]>::len);
		self.pl_selection = self.pl_selection.min(len.saturating_sub(1));
	}

	fn clamp_job(&mut self) {
		let len = self.jobs_slice().map_or(0, <[_]>::len);
		self.job_selection =
			self.job_selection.min(len.saturating_sub(1));
	}

	fn move_selection(&mut self, down: bool) {
		match self.level() {
			Level::Pipelines => {
				let len =
					self.pipelines_slice().map_or(0, <[_]>::len);
				self.pl_selection =
					step(self.pl_selection, len, down);
			}
			Level::Jobs => {
				let len = self.jobs_slice().map_or(0, <[_]>::len);
				self.job_selection =
					step(self.job_selection, len, down);
			}
			Level::Trace => {
				if down {
					self.trace_scroll =
						self.trace_scroll.saturating_add(1);
				} else {
					self.trace_scroll =
						self.trace_scroll.saturating_sub(1);
				}
			}
		}
	}

	fn draw_message(&self, f: &mut Frame, rect: Rect, msg: &str) {
		let block = Block::default()
			.borders(Borders::ALL)
			.title(self.title());
		f.render_widget(
			Paragraph::new(msg)
				.block(block)
				.alignment(Alignment::Center)
				.style(self.theme.text(true, false)),
			rect,
		);
	}

	fn title(&self) -> String {
		self.remote.as_ref().map_or_else(
			|| "Pipelines".to_string(),
			|r| format!("Pipelines · {}", r.project_path),
		)
	}

	fn host(&self) -> &str {
		self.remote.as_ref().map_or("", |r| r.host.as_str())
	}

	fn split_footer(&self, rect: Rect) -> (Rect, Option<Rect>) {
		if self.status_msg.is_some() {
			let chunks = Layout::default()
				.direction(Direction::Vertical)
				.constraints([
					Constraint::Min(1),
					Constraint::Length(1),
				])
				.split(rect);
			(chunks[0], Some(chunks[1]))
		} else {
			(rect, None)
		}
	}

	fn draw_footer(&self, f: &mut Frame, footer: Option<Rect>) {
		if let (Some(rect), Some(msg)) =
			(footer, self.status_msg.as_deref())
		{
			f.render_widget(
				Paragraph::new(msg)
					.style(self.theme.text(true, false)),
				rect,
			);
		}
	}

	fn render_pipelines(
		&self,
		f: &mut Frame,
		rect: Rect,
		pipelines: &[Pipeline],
	) {
		let (area, footer) = self.split_footer(rect);
		let items: Vec<ListItem> = pipelines
			.iter()
			.enumerate()
			.map(|(i, p)| {
				let git_ref =
					p.r#ref.as_deref().unwrap_or("");
				ListItem::new(Line::from(vec![Span::styled(
					format!(
						"{} #{}  {}  {}",
						ci_marker(p.status),
						p.id,
						status_label(p.status),
						git_ref,
					),
					self.theme.text(true, i == self.pl_selection),
				)]))
			})
			.collect();
		f.render_widget(
			List::new(items).block(
				Block::default()
					.borders(Borders::ALL)
					.title(format!(
						"{}  ·  [enter] jobs  [t] retry  [x] cancel  [o] open  [r] refresh",
						self.title()
					)),
			),
			area,
		);
		self.draw_footer(f, footer);
	}

	fn render_jobs(&self, f: &mut Frame, rect: Rect, jobs: &[Job]) {
		let (area, footer) = self.split_footer(rect);
		let items: Vec<ListItem> = jobs
			.iter()
			.enumerate()
			.map(|(i, j)| {
				ListItem::new(Line::from(vec![Span::styled(
					format!(
						"{} {}  [{}]  {}",
						ci_marker(j.status),
						j.name,
						j.stage,
						status_label(j.status),
					),
					self.theme.text(true, i == self.job_selection),
				)]))
			})
			.collect();
		let pid =
			self.jobs_pipeline_id.map_or(0, |id| id);
		f.render_widget(
			List::new(items).block(
				Block::default().borders(Borders::ALL).title(
					format!(
						"Pipeline #{pid} jobs  ·  [enter] trace  [t] retry  [x] cancel  [o] open  [esc] back"
					),
				),
			),
			area,
		);
		self.draw_footer(f, footer);
	}

	fn render_trace(&self, f: &mut Frame, rect: Rect, trace: &str) {
		let (area, footer) = self.split_footer(rect);
		let jid = self.trace_job_id.map_or(0, |id| id);
		f.render_widget(
			Paragraph::new(trace)
				.block(
					Block::default().borders(Borders::ALL).title(
						format!(
							"Job #{jid} trace  ·  [↑/↓] scroll  [esc] back"
						),
					),
				)
				.wrap(Wrap { trim: false })
				.style(self.theme.text(true, false))
				.scroll((self.trace_scroll, 0)),
			area,
		);
		self.draw_footer(f, footer);
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
	Pipelines,
	Jobs,
	Trace,
}

fn step(current: usize, len: usize, down: bool) -> usize {
	if len == 0 {
		return 0;
	}
	if down {
		(current + 1) % len
	} else {
		current.checked_sub(1).unwrap_or(len - 1)
	}
}

const fn ci_marker(status: CiStatus) -> &'static str {
	match status {
		CiStatus::Success => "✓",
		CiStatus::Failed => "✗",
		CiStatus::Running => "▶",
		CiStatus::Canceled => "⊘",
		CiStatus::Skipped => "»",
		CiStatus::Manual => "⏸",
		CiStatus::Unknown => "?",
		_ => "•",
	}
}

const fn status_label(status: CiStatus) -> &'static str {
	match status {
		CiStatus::Created => "created",
		CiStatus::WaitingForResource => "waiting",
		CiStatus::Preparing => "preparing",
		CiStatus::Pending => "pending",
		CiStatus::Running => "running",
		CiStatus::Success => "success",
		CiStatus::Failed => "failed",
		CiStatus::Canceled => "canceled",
		CiStatus::Skipped => "skipped",
		CiStatus::Manual => "manual",
		CiStatus::Scheduled => "scheduled",
		CiStatus::Unknown => "?",
	}
}

impl DrawableComponent for PipelinesTab {
	fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()> {
		if self.remote.is_none() {
			self.draw_message(
				f,
				rect,
				"No GitLab remote detected for this repository.",
			);
			return Ok(());
		}

		if !self.token_available() {
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
			} else if let Some(err) = &self.token_error {
				self.draw_message(
					f,
					rect,
					&format!(
						"{err}\n\nPress [Enter] to try again."
					),
				);
			} else {
				self.draw_message(
					f,
					rect,
					&strings::gitlab_token_help(self.host(), true),
				);
			}
			return Ok(());
		}

		match self.level() {
			Level::Trace => match self.trace.as_ref() {
				Some(Load::Loading) => {
					self.draw_message(f, rect, "Loading trace…");
				}
				Some(Load::Error(e)) => self.draw_message(
					f,
					rect,
					&format!("Failed to load trace:\n{e}"),
				),
				Some(Load::Loaded(t)) => {
					self.render_trace(f, rect, t);
				}
				None => {}
			},
			Level::Jobs => match self.jobs.as_ref() {
				Some(Load::Loading) => {
					self.draw_message(f, rect, "Loading jobs…");
				}
				Some(Load::Error(e)) => self.draw_message(
					f,
					rect,
					&format!("Failed to load jobs:\n{e}"),
				),
				Some(Load::Loaded(j)) if j.is_empty() => {
					self.draw_message(f, rect, "No jobs.");
				}
				Some(Load::Loaded(j)) => {
					self.render_jobs(f, rect, j);
				}
				None => {}
			},
			Level::Pipelines => match &self.pipelines {
				Load::Loading => {
					self.draw_message(
						f,
						rect,
						"Loading pipelines…",
					);
				}
				Load::Error(e) => self.draw_message(
					f,
					rect,
					&format!(
						"Failed to load pipelines:\n{e}\n\nPress [r] to retry."
					),
				),
				Load::Loaded(p) if p.is_empty() => {
					self.draw_message(f, rect, "No pipelines.");
				}
				Load::Loaded(p) => {
					self.render_pipelines(f, rect, p);
				}
			},
		}

		Ok(())
	}
}

impl Component for PipelinesTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			let has_sel = self.selected_pipeline().is_some()
				|| self.selected_job().is_some();
			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				true,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::pipeline_open(&self.key_config),
				has_sel,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::pipeline_retry(&self.key_config),
				has_sel,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::pipeline_cancel(&self.key_config),
				has_sel,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::gitlab_browser(&self.key_config),
				has_sel,
				true,
			));
		}
		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if !self.is_visible() {
			return Ok(EventState::NotConsumed);
		}

		if self.token_input.is_visible() {
			if !self.token_input.event(ev)?.is_consumed() {
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
			}
			return Ok(EventState::Consumed);
		}

		if let Event::Key(k) = ev {
			if self.key_event(k) {
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

impl PipelinesTab {
	/// Returns true when the key was consumed.
	fn key_event(&mut self, k: &KeyEvent) -> bool {
		let token_missing = !self.token_available();

		if key_match(k, self.key_config.keys.move_down) {
			self.move_selection(true);
		} else if key_match(k, self.key_config.keys.move_up) {
			self.move_selection(false);
		} else if key_match(k, self.key_config.keys.exit_popup)
			&& self.level() != Level::Pipelines
		{
			self.go_back();
		} else if key_match(k, self.key_config.keys.enter) {
			if token_missing {
				self.show_token_prompt();
			} else {
				match self.level() {
					Level::Pipelines => self.open_jobs(),
					Level::Jobs => self.open_trace(),
					Level::Trace => {}
				}
			}
		} else if matches!(k.code, KeyCode::Char('t')) {
			self.retry_selected();
		} else if matches!(k.code, KeyCode::Char('x')) {
			self.cancel_selected();
		} else if matches!(k.code, KeyCode::Char('o')) {
			self.open_in_browser();
		} else if matches!(k.code, KeyCode::Char('r')) && !token_missing
		{
			self.reload();
		} else {
			return false;
		}
		true
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
