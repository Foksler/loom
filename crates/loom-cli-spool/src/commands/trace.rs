// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct TraceArgs {
	pub revset: Option<String>,
}

pub async fn run(_args: TraceArgs) -> anyhow::Result<()> {
	todo!()
}
