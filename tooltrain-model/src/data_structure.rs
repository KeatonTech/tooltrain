use std::{ops::Deref, path::PathBuf};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::events::ToolchainModelMutation;
use crate::Extras;

pub type GraphSize = u32;
pub type ToolOutletId = String;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ToolId(pub(crate) GraphSize);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireRef {
    pub id: GraphSize,
    pub from_tool_id: ToolId,
    pub output: ToolOutletId,
    pub to_tool_id: ToolId,
    pub input: ToolOutletId,
}

impl From<WireRef> for petgraph::graph::EdgeIndex<GraphSize> {
    fn from(wire_id: WireRef) -> Self {
        petgraph::graph::EdgeIndex::new(wire_id.id as usize)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolReference {
    pub name: String,
    pub description: String,
    pub path: Option<PathBuf>,
    pub hash: [u8; 32],
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ToolOutletReference {
    pub tool_outlet_id: ToolOutletId,
    pub name: String,
    pub description: String,
    pub data_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolOutlet {
    pub outlet: ToolOutletReference,
    pub tool_id: ToolId,
    pub extras: Extras,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolInstance {
    pub id: ToolId,
    pub tool: ToolReference,
    pub extras: Extras,
}

#[derive(Clone, Debug)]
pub struct TooltrainChangeSender(broadcast::Sender<ToolchainModelMutation>);

impl Default for TooltrainChangeSender {
    fn default() -> Self {
        TooltrainChangeSender(broadcast::channel((u32::MAX / 2) as usize).0)
    }
}

impl Deref for TooltrainChangeSender {
    type Target = broadcast::Sender<ToolchainModelMutation>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

mod internal {}
