// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StitchId(pub [u8; 16]);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TreeId(pub [u8; 20]);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationId(pub [u8; 16]);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature {
	pub name: String,
	pub email: String,
	pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stitch {
	pub id: StitchId,
	pub parents: Vec<StitchId>,
	pub tree_id: TreeId,
	pub description: String,
	pub author: Signature,
	pub committer: Signature,
	pub is_knotted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pin {
	pub name: String,
	pub target: StitchId,
	pub is_tracking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TangleSide {
	Ours,
	Theirs,
	Base,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tangle {
	pub path: PathBuf,
	pub sides: Vec<TangleSide>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shuttle {
	pub stitch_id: StitchId,
	pub tree_state: TreeId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TensionEntry {
	pub operation_id: OperationId,
	pub timestamp: DateTime<Utc>,
	pub description: String,
}
