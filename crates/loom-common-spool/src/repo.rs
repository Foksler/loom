// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::path::Path;

use crate::error::Result;
use crate::types::{Stitch, StitchId, Tangle, TangleSide};

pub struct SpoolRepo {
	_inner: gix::Repository,
}

impl SpoolRepo {
	pub fn wind(path: impl AsRef<Path>, _colocate_git: bool) -> Result<Self> {
		let _ = path;
		todo!()
	}

	pub fn open(path: impl AsRef<Path>) -> Result<Self> {
		let _ = path;
		todo!()
	}

	pub fn stitch(&self) -> Result<StitchId> {
		todo!()
	}

	pub fn knot(&self, _message: &str) -> Result<()> {
		todo!()
	}

	pub fn mark(&self, _id: &StitchId, _message: &str) -> Result<()> {
		todo!()
	}

	pub fn trace(&self, _revset: &str) -> Result<Vec<Stitch>> {
		todo!()
	}

	pub fn rethread(&self, _source: &StitchId, _dest: &StitchId) -> Result<()> {
		todo!()
	}

	pub fn ply(&self, _source: &StitchId, _dest: &StitchId) -> Result<()> {
		todo!()
	}

	pub fn snip(&self, _id: &StitchId) -> Result<()> {
		todo!()
	}

	pub fn tangles(&self) -> Result<Vec<Tangle>> {
		todo!()
	}

	pub fn untangle(&self, _path: impl AsRef<Path>, _resolution: TangleSide) -> Result<()> {
		todo!()
	}

	pub fn unpick(&self) -> Result<()> {
		todo!()
	}

	pub fn shuttle(&self, _remote: &str, _pins: &[String]) -> Result<()> {
		todo!()
	}

	pub fn draw(&self, _remote: &str) -> Result<()> {
		todo!()
	}
}
