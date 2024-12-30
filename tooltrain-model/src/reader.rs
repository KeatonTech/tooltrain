use crate::{
    internal::TooltrainModelInternal, Extras, ToolId, ToolInstance, ToolOutlet, ToolOutletId,
    WireRef,
};
use serde::Serialize;

pub(crate) trait TooltrainModelRef {
    fn as_ref(&self) -> &TooltrainModelInternal;
}

pub struct TooltrainModelReadLock<'a>(tokio::sync::RwLockReadGuard<'a, TooltrainModelInternal>);

impl<'a> Serialize for TooltrainModelReadLock<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'a> From<tokio::sync::RwLockReadGuard<'a, TooltrainModelInternal>>
    for TooltrainModelReadLock<'a>
{
    fn from(guard: tokio::sync::RwLockReadGuard<'a, TooltrainModelInternal>) -> Self {
        Self(guard)
    }
}

impl<'a> TooltrainModelRef for TooltrainModelReadLock<'a> {
    fn as_ref(&self) -> &TooltrainModelInternal {
        &self.0
    }
}

pub trait TooltrainModelReader: Serialize {
    fn get_tool(&self, tool_id: ToolId) -> Option<&ToolInstance>;
    fn tools(&self) -> impl Iterator<Item = (ToolId, &ToolInstance)>;
    fn inputs_for_tool(&self, tool_id: ToolId)
        -> Option<impl Iterator<Item = &'_ ToolOutlet> + '_>;
    fn get_input(&self, tool_id: ToolId, input_id: &ToolOutletId) -> Option<&ToolOutlet>;
    fn get_input_extras(&self, tool_id: ToolId, input_id: &ToolOutletId) -> Option<&Extras>;
    fn wires_for_input(
        &self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<impl Iterator<Item = WireRef> + '_>;
    fn outputs_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = &'_ ToolOutlet> + '_>;
    fn get_output(&self, tool_id: ToolId, output_id: &ToolOutletId) -> Option<&ToolOutlet>;
    fn get_output_extras(&self, tool_id: ToolId, output_id: &ToolOutletId) -> Option<&Extras>;
    fn wires_for_output(
        &self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<impl Iterator<Item = WireRef> + '_>;
    fn get_wire_extras(&self, wire: WireRef) -> Option<&Extras>;
}

impl<T: TooltrainModelRef + Serialize> TooltrainModelReader for T {
    fn get_tool(&self, tool_id: ToolId) -> Option<&ToolInstance> {
        self.as_ref().get_tool(tool_id)
    }

    fn tools(&self) -> impl Iterator<Item = (ToolId, &ToolInstance)> {
        self.as_ref().tools()
    }

    fn inputs_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = &'_ ToolOutlet> + '_> {
        self.as_ref().inputs_for_tool(tool_id)
    }

    fn get_input(&self, tool_id: ToolId, input_id: &ToolOutletId) -> Option<&ToolOutlet> {
        self.as_ref().get_input(tool_id, input_id)
    }

    fn get_input_extras(&self, tool_id: ToolId, input_id: &ToolOutletId) -> Option<&Extras> {
        self.as_ref().get_input_extras(tool_id, input_id)
    }

    fn wires_for_input(
        &self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<impl Iterator<Item = WireRef> + '_> {
        self.as_ref().wires_for_input(tool_id, input_id)
    }

    fn outputs_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = &'_ ToolOutlet> + '_> {
        self.as_ref().outputs_for_tool(tool_id)
    }

    fn get_output(&self, tool_id: ToolId, output_id: &ToolOutletId) -> Option<&ToolOutlet> {
        self.as_ref().get_output(tool_id, output_id)
    }

    fn get_output_extras(&self, tool_id: ToolId, output_id: &ToolOutletId) -> Option<&Extras> {
        self.as_ref().get_output_extras(tool_id, output_id)
    }

    fn wires_for_output(
        &self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<impl Iterator<Item = WireRef> + '_> {
        self.as_ref().wires_for_output(tool_id, output_id)
    }

    fn get_wire_extras(&self, wire: WireRef) -> Option<&Extras> {
        self.as_ref().get_wire_extras(wire)
    }
}
