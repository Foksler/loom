// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::path::PathBuf;

#[derive(Debug, Clone, clap::Args)]
pub struct MendArgs {
	pub path: PathBuf,
}

pub async fn run(_args: MendArgs) -> anyhow::Result<()> {
	todo!()
}
