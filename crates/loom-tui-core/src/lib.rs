// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::borrow::Cow;

use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use thiserror::Error;

/// Result type alias using ComponentError as the default error type.
pub type Result<T, E = ComponentError> = std::result::Result<T, E>;

/// Type alias for focus identifiers.
pub type FocusId = String;

/// Actions that components can emit in response to events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
	/// Periodic tick for animations or polling.
	Tick,
	/// Request application shutdown.
	Quit,
	/// Request a re-render of the UI.
	Render,
	/// Terminal was resized to (width, height).
	Resize(u16, u16),
	/// Move focus to the next focusable component.
	FocusNext,
	/// Move focus to the previous focusable component.
	FocusPrev,
	/// Scroll up by the specified number of lines.
	ScrollUp(usize),
	/// Scroll down by the specified number of lines.
	ScrollDown(usize),
	/// Submit/confirm the current input or selection.
	Submit,
	/// Cancel the current operation.
	Cancel,
	/// Custom action with a kind identifier and payload.
	Custom { kind: Cow<'static, str>, payload: String },
}

/// Terminal events from crossterm.
#[derive(Debug, Clone)]
pub enum Event {
	/// Keyboard input.
	Key(KeyEvent),
	/// Mouse input.
	Mouse(MouseEvent),
	/// Terminal resize to (width, height).
	Resize(u16, u16),
	/// Periodic tick.
	Tick,
	/// Paste event with pasted text.
	Paste(String),
	/// Terminal gained focus.
	FocusGained,
	/// Terminal lost focus.
	FocusLost,
}

impl From<crossterm::event::Event> for Event {
	fn from(event: crossterm::event::Event) -> Self {
		match event {
			crossterm::event::Event::Key(key) => Event::Key(key),
			crossterm::event::Event::Mouse(mouse) => Event::Mouse(mouse),
			crossterm::event::Event::Resize(w, h) => Event::Resize(w, h),
			crossterm::event::Event::Paste(text) => Event::Paste(text),
			crossterm::event::Event::FocusGained => Event::FocusGained,
			crossterm::event::Event::FocusLost => Event::FocusLost,
		}
	}
}

/// Outcome of event handling, indicating whether the event was consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventOutcome<A> {
	/// Event was not handled; propagate to other handlers.
	Ignored,
	/// Event was handled, optionally producing an action.
	Handled(Option<A>),
}

/// Tracks which component currently has focus.
#[derive(Debug, Default, Clone)]
pub struct FocusState {
	pub focused_id: Option<FocusId>,
	pub focusable_ids: Vec<FocusId>,
}

impl FocusState {
	/// Register a new focusable component.
	pub fn register(&mut self, id: FocusId) {
		if !self.focusable_ids.contains(&id) {
			self.focusable_ids.push(id);
		}
	}

	/// Unregister a focusable component.
	pub fn unregister(&mut self, id: &str) {
		self.focusable_ids.retain(|i| i != id);
		if self.focused_id.as_deref() == Some(id) {
			self.focused_id = None;
		}
	}

	/// Move focus to the next component.
	pub fn focus_next(&mut self) {
		if self.focusable_ids.is_empty() {
			return;
		}
		let next_idx = match self.focused_index() {
			Some(idx) => (idx + 1) % self.focusable_ids.len(),
			None => 0,
		};
		self.focused_id = Some(self.focusable_ids[next_idx].clone());
	}

	/// Move focus to the previous component.
	pub fn focus_prev(&mut self) {
		if self.focusable_ids.is_empty() {
			return;
		}
		let prev_idx = match self.focused_index() {
			Some(idx) => {
				if idx == 0 {
					self.focusable_ids.len() - 1
				} else {
					idx - 1
				}
			}
			None => self.focusable_ids.len() - 1,
		};
		self.focused_id = Some(self.focusable_ids[prev_idx].clone());
	}

	/// Set focus to a specific component by id.
	pub fn set_focus(&mut self, id: &str) {
		if self.focusable_ids.iter().any(|i| i == id) {
			self.focused_id = Some(id.to_string());
		}
	}

	/// Check if a component has focus.
	pub fn is_focused(&self, id: &str) -> bool {
		self.focused_id.as_deref() == Some(id)
	}

	fn focused_index(&self) -> Option<usize> {
		self.focused_id
			.as_ref()
			.and_then(|id| self.focusable_ids.iter().position(|i| i == id))
	}
}

/// Trait for mapping key events to actions based on focus state.
pub trait Keymap<A> {
	fn key_to_action(&self, key: &KeyEvent, focus: &FocusState) -> Option<A>;
}

#[async_trait]
pub trait EventSource {
	async fn next(&mut self) -> Option<Event>;
}

#[derive(Debug, Error)]
pub enum ComponentError {
	#[error("initialization failed: {0}")]
	Init(String),
	#[error("render failed: {0}")]
	Render(String),
}
