// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct RethreadArgs {
	pub source: String,
	pub dest: String,
}

pub async fn run(_args: RethreadArgs) -> anyhow::Result<()> {
	todo!()
}
