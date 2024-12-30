use crate::{
    events::ToolchainModelMutation, internal::TooltrainModelInternal, Extras, ToolId, ToolInstance,
    ToolOutlet, ToolOutletId, ToolOutletReference, TooltrainModel, TooltrainModelRef, WireRef,
};
use anyhow::Error;
use std::ops::Deref;
use tokio::sync::RwLockWriteGuard;

pub struct TooltrainModelMutator<'a>(RwLockWriteGuard<'a, TooltrainModelInternal>);

impl TooltrainModelRef for TooltrainModelMutator<'_> {
    fn as_ref(&self) -> &TooltrainModelInternal {
        self.0.deref()
    }
}

impl<'a> TooltrainModelMutator<'a> {
    pub(crate) async fn new(model: &'a TooltrainModel) -> Self {
        TooltrainModelMutator(model.0.write().await)
    }

    /** Adds a new tool instance to the model and returns its id. */
    pub fn add_tool_instance(&mut self, tool_instance: ToolInstance) -> ToolId {
        let tool_instance = self.0.add_tool(tool_instance);
        self.0
            .mutations
            .send(ToolchainModelMutation::AddedToolInstance(
                tool_instance.clone(),
            ))
            .unwrap();
        tool_instance.id
    }

    /** Removes a tool instance from the model. */
    pub fn remove_tool_instance(&mut self, tool_id: ToolId) -> Option<ToolInstance> {
        let tool_instance = self.0.remove_tool(tool_id);
        if tool_instance.is_some() {
            self.0
                .mutations
                .send(ToolchainModelMutation::RemovedToolInstance(tool_id))
                .unwrap();
        }
        tool_instance
    }

    pub fn add_input(
        &mut self,
        tool_id: ToolId,
        input: ToolOutletReference,
    ) -> Result<ToolOutlet, Error> {
        let outlet = self
            .0
            .add_input_for_tool(tool_id, input, Default::default())?;
        self.0
            .mutations
            .send(ToolchainModelMutation::AddedInput(outlet.clone()))
            .unwrap();
        Ok(outlet)
    }

    pub fn remove_input(&mut self, tool_id: ToolId, input_id: ToolOutletId) -> Option<ToolOutlet> {
        let removed_input = self.0.remove_input_for_tool(tool_id, &input_id)?;
        self.0
            .mutations
            .send(ToolchainModelMutation::RemovedInput {
                tool: tool_id,
                input: input_id,
            })
            .unwrap();
        Some(removed_input)
    }

    pub fn add_output(
        &mut self,
        tool_id: ToolId,
        output: ToolOutletReference,
    ) -> Result<ToolOutlet, Error> {
        let outlet = self
            .0
            .add_output_for_tool(tool_id, output, Default::default())?;
        self.0
            .mutations
            .send(ToolchainModelMutation::AddedOutput(outlet.clone()))
            .unwrap();
        Ok(outlet)
    }

    pub fn remove_output(
        &mut self,
        tool_id: ToolId,
        output_id: ToolOutletId,
    ) -> Option<ToolOutlet> {
        let removed_output = self.0.remove_output_for_tool(tool_id, &output_id)?;
        self.0
            .mutations
            .send(ToolchainModelMutation::RemovedOutput {
                tool: tool_id,
                input: output_id,
            })
            .unwrap();
        Some(removed_output)
    }

    pub fn add_wire(
        &mut self,
        from_tool_id: ToolId,
        output_id: ToolOutletId,
        to_tool_id: ToolId,
        input_id: ToolOutletId,
    ) -> Result<WireRef, Error> {
        self.add_wire_with_extras(
            from_tool_id,
            output_id,
            to_tool_id,
            input_id,
            Default::default(),
        )
    }

    pub fn add_wire_with_extras(
        &mut self,
        from_tool_id: ToolId,
        output_id: ToolOutletId,
        to_tool_id: ToolId,
        input_id: ToolOutletId,
        extras: Extras,
    ) -> Result<WireRef, Error> {
        let wire = self
            .0
            .add_wire(from_tool_id, output_id, to_tool_id, input_id, extras)?;
        self.0
            .mutations
            .send(ToolchainModelMutation::AddedWire(wire.clone()))
            .unwrap();
        Ok(wire)
    }

    pub fn remove_wire(&mut self, wire_ref: WireRef) -> Option<()> {
        self.0.remove_wire(wire_ref.clone())?;
        self.0
            .mutations
            .send(ToolchainModelMutation::RemovedWire(wire_ref))
            .unwrap();
        Some(())
    }
}
