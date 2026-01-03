// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#[derive(Debug, Clone, clap::Args)]
pub struct ShowArgs {
	pub stitch: Option<String>,
}

pub async fn run(_args: ShowArgs) -> anyhow::Result<()> {
	todo!()
}
