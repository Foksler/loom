// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct PinArgs {
	#[command(subcommand)]
	pub command: PinSubcommand,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum PinSubcommand {
	List,
	Create { name: String },
	Delete { name: String },
}

pub async fn run(_args: PinArgs) -> anyhow::Result<()> {
	todo!()
}
