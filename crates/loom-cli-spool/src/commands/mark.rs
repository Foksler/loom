// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct MarkArgs {
	#[arg(short, long)]
	pub message: Option<String>,
}

pub async fn run(_args: MarkArgs) -> anyhow::Result<()> {
	todo!()
}
