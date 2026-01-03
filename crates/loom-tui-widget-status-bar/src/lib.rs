// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use loom_tui_theme::Theme;
use ratatui::{
	buffer::Buffer,
	layout::Rect,
	style::{Style, Stylize},
	text::{Line, Span},
	widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct StatusItem {
	pub label: String,
	pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct StatusBar {
	items: Vec<StatusItem>,
	shortcuts: Vec<(String, String)>,
	style: Style,
}

impl StatusBar {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn item(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
		self.items.push(StatusItem {
			label: label.into(),
			value: value.into(),
		});
		self
	}

	pub fn shortcut(mut self, key: impl Into<String>, desc: impl Into<String>) -> Self {
		self.shortcuts.push((key.into(), desc.into()));
		self
	}

	pub fn style(mut self, style: Style) -> Self {
		self.style = style;
		self
	}
}

impl Widget for StatusBar {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let theme = Theme::default();

		if self.style != Style::default() {
			buf.set_style(area, self.style);
		}

		let mut right_spans = Vec::new();
		for (i, (key, desc)) in self.shortcuts.iter().enumerate() {
			if i > 0 {
				right_spans.push(Span::raw(" | "));
			}
			right_spans.push(Span::raw(key).bold().fg(theme.colors.accent));
			right_spans.push(Span::raw(" "));
			right_spans.push(Span::raw(desc));
		}
		let right_line = Line::from(right_spans);
		let right_width = right_line.width() as u16;

		let available_for_left = area.width.saturating_sub(right_width + 1);

		let mut left_spans = Vec::new();
		let mut left_total_width = 0usize;
		for (i, item) in self.items.iter().enumerate() {
			let separator = if i > 0 { " | " } else { "" };
			let item_str = format!("{}{}: {}", separator, item.label, item.value);
			let item_width = item_str.len();

			if left_total_width + item_width > available_for_left as usize {
				let remaining = available_for_left as usize - left_total_width;
				if remaining > 3 {
					let truncated: String = item_str.chars().take(remaining.saturating_sub(1)).collect();
					if i > 0 {
						left_spans.push(Span::raw(" | "));
					}
					left_spans.push(Span::raw(format!("{}…", truncated.trim_start_matches(" | "))));
				}
				break;
			}

			if i > 0 {
				left_spans.push(Span::raw(" | "));
			}
			left_spans.push(Span::raw(&item.label).bold());
			left_spans.push(Span::raw(": "));
			left_spans.push(Span::raw(&item.value));
			left_total_width += item_width;
		}

		let left_line = Line::from(left_spans);
		buf.set_line(area.x, area.y, &left_line, available_for_left);

		let right_x = area.right().saturating_sub(right_width);
		if right_x > area.x {
			buf.set_line(right_x, area.y, &right_line, right_width);
		}
	}
}
