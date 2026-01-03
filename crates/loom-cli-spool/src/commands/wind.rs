// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct WindArgs {
	#[arg(long)]
	pub git: bool,
}

pub async fn run(_args: WindArgs) -> anyhow::Result<()> {
	todo!()
}
