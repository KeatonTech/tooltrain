use serde::{Deserialize, Serialize};

use crate::{ToolId, ToolInstance, ToolOutlet, ToolOutletId, WireRef};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ToolchainModelMutation {
    AddedToolInstance(ToolInstance),
    RemovedToolInstance(ToolId),
    AddedInput(ToolOutlet),
    RemovedInput {
        tool: ToolId,
        input: ToolOutletId,
    },
    AddedOutput(ToolOutlet),
    RemovedOutput {
        tool: ToolId,
        input: ToolOutletId,
    },
    ChangedToolInstanceExtraValue {
        tool: ToolId,
        key: String,
        new_value: Option<Vec<u8>>,
    },

    AddedWire(WireRef),
    RemovedWire(WireRef),
    ChangedWireExtraValue {
        wire: WireRef,
        key: String,
        new_value: Option<Vec<u8>>,
    },

    ChangedModelExtraValue {
        key: String,
        new_value: Option<Vec<u8>>,
    },
}
