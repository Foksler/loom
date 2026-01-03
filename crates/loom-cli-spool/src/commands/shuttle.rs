// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct ShuttleArgs {
	pub remote: Option<String>,
	#[arg(long)]
	pub pins: Vec<String>,
}

pub async fn run(_args: ShuttleArgs) -> anyhow::Result<()> {
	todo!()
}
