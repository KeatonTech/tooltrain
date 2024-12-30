use crate::{
    Extras, GraphSize, ToolId, ToolInstance, ToolOutlet, ToolOutletId, ToolOutletReference,
    TooltrainChangeSender, WireRef,
};
use anyhow::{anyhow, Error};
use petgraph::{
    matrix_graph::NodeIndex, prelude::StableGraph, visit::EdgeRef, Directed, Direction,
};
use serde::{Deserialize, Serialize};

impl From<ToolId> for NodeIndex<GraphSize> {
    fn from(tool_id: ToolId) -> Self {
        NodeIndex::new(tool_id.0 as usize)
    }
}

impl From<NodeIndex<GraphSize>> for ToolId {
    fn from(node_index: NodeIndex<GraphSize>) -> Self {
        ToolId(node_index.index() as GraphSize)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum NodeData {
    Input(ToolOutlet),
    Tool(ToolInstance),
    Output(ToolOutlet),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WireData {
    ToolConnection(Extras),
    OutletConnection(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct TooltrainModelInternal {
    graph: StableGraph<NodeData, WireData, Directed, GraphSize>,

    pub(crate) extras: Extras,

    #[serde(skip)]
    pub(crate) mutations: TooltrainChangeSender,
}

/// Implements a basic CRUD interface for the internal graph representation.
///
/// Important: This impl only validates that the graph is structurally correct (ie.
/// Inputs leading to Tools leading to Outputs). It does no business logic validation.
/// Basically, this API will let you create un-runnable tooltrain programs, but it
/// won't let you create corrupted or unreadable ones.
impl TooltrainModelInternal {
    pub(crate) fn add_tool(&mut self, tool: ToolInstance) -> ToolInstance {
        let tool_node_id = self.graph.add_node(NodeData::Tool(tool));
        let stored_tool_instance = self.graph.node_weight_mut(tool_node_id);
        if let Some(NodeData::Tool(tool)) = stored_tool_instance {
            tool.id = tool_node_id.into();
            return tool.clone();
        } else {
            panic!("Expected tool node");
        }
    }

    pub(crate) fn remove_tool(&mut self, tool_id: ToolId) -> Option<ToolInstance> {
        self.graph
            .remove_node(tool_id.into())
            .map(|node| match node {
                NodeData::Tool(tool) => tool,
                _ => panic!("Expected tool node"),
            })
    }

    pub(crate) fn get_tool(&self, tool_id: ToolId) -> Option<&ToolInstance> {
        self.graph
            .node_weight(tool_id.into())
            .and_then(|node| match node {
                NodeData::Tool(tool) => Some(tool),
                _ => panic!("Expected tool node"),
            })
    }

    pub(crate) fn get_tool_mut(&mut self, tool_id: ToolId) -> Option<&mut ToolInstance> {
        self.graph
            .node_weight_mut(tool_id.into())
            .and_then(|node| match node {
                NodeData::Tool(tool) => Some(tool),
                _ => panic!("Expected tool node"),
            })
    }

    pub(crate) fn tools(&self) -> impl Iterator<Item = (ToolId, &ToolInstance)> {
        self.graph
            .node_indices()
            .filter_map(move |node_id| match &self.graph[node_id] {
                NodeData::Tool(tool) => Some((node_id.into(), tool)),
                _ => None,
            })
    }

    pub(crate) fn assert_tool_exists(&self, tool_id: ToolId) -> Result<(), Error> {
        if !self.graph.contains_node(tool_id.into()) {
            return Err(anyhow!("Tool {:?} does not exist", tool_id));
        }
        debug_assert!(matches!(
            self.graph[NodeIndex::from(tool_id)],
            NodeData::Tool(_)
        ));
        Ok(())
    }

    pub(crate) fn input_nodes_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = NodeIndex<GraphSize>> + '_> {
        self.assert_tool_exists(tool_id).ok()?;
        Some(
            self.graph
                .neighbors_directed(tool_id.into(), Direction::Incoming),
        )
    }

    pub(crate) fn inputs_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = &'_ ToolOutlet> + '_> {
        self.input_nodes_for_tool(tool_id).map(|node_ids| {
            node_ids.map(|node_id| match &self.graph[node_id] {
                NodeData::Input(outlet) => outlet,
                _ => panic!("Expected input node"),
            })
        })
    }

    pub(crate) fn get_input_node(
        &self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<NodeIndex<GraphSize>> {
        self.assert_tool_exists(tool_id).ok()?;
        self.graph
            .edges_directed(tool_id.into(), Direction::Incoming)
            .find_map(|edge| {
                if let WireData::OutletConnection(edge_input_id) = edge.weight() {
                    if edge_input_id == input_id {
                        debug_assert!(matches!(self.graph[edge.source()], NodeData::Input(_)));
                        Some(edge.source())
                    } else {
                        None
                    }
                } else {
                    panic!("Expected outlet connection");
                }
            })
    }

    pub(crate) fn get_input(
        &self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<&ToolOutlet> {
        self.get_input_node(tool_id, input_id)
            .map(|node_id| match &self.graph[node_id] {
                NodeData::Input(outlet) => outlet,
                _ => panic!("Expected input node"),
            })
    }

    pub(crate) fn add_input_for_tool(
        &mut self,
        tool_id: ToolId,
        outlet_reference: ToolOutletReference,
        extras: Extras,
    ) -> Result<ToolOutlet, Error> {
        self.assert_tool_exists(tool_id)?;
        let outlet = ToolOutlet {
            outlet: outlet_reference,
            tool_id,
            extras,
        };
        let input_node = self.graph.add_node(NodeData::Input(outlet.clone()));
        self.graph.add_edge(
            input_node,
            tool_id.into(),
            WireData::OutletConnection(outlet.outlet.tool_outlet_id.clone()),
        );
        Ok(outlet)
    }

    pub(crate) fn remove_input_for_tool(
        &mut self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<ToolOutlet> {
        self.assert_tool_exists(tool_id).ok()?;
        let input_node = self.get_input_node(tool_id, input_id)?;
        match self.graph.remove_node(input_node) {
            Some(NodeData::Input(outlet)) => Some(outlet),
            None => panic!("Node was removed after lookup"),
            _ => panic!("Expected input node"),
        }
    }

    pub(crate) fn get_input_extras(
        &self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<&Extras> {
        self.get_input_node(tool_id, input_id)
            .map(|node_id| match &self.graph[node_id] {
                NodeData::Input(outlet) => &outlet.extras,
                _ => panic!("Expected input node"),
            })
    }

    pub(crate) fn get_input_extras_mut(
        &mut self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<&mut Extras> {
        self.get_input_node(tool_id, input_id)
            .map(move |node_id| match &mut self.graph[node_id] {
                NodeData::Input(outlet) => &mut outlet.extras,
                _ => panic!("Expected input node"),
            })
    }

    pub(crate) fn wires_for_input(
        &self,
        tool_id: ToolId,
        input_id: &ToolOutletId,
    ) -> Option<impl Iterator<Item = WireRef> + '_> {
        let cloned_input_id = input_id.clone();
        self.get_input_node(tool_id, input_id)
            .map(move |input_node| {
                self.graph
                    .edges_directed(input_node, Direction::Incoming)
                    .map(move |edge| {
                        let source_node = edge.source();
                        let (source_tool_id, source_output_id) = match &self.graph[source_node] {
                            NodeData::Output(output) => {
                                (output.tool_id, output.outlet.tool_outlet_id.clone())
                            }
                            _ => panic!("Expected output node"),
                        };
                        WireRef {
                            id: edge.id().index() as GraphSize,
                            from_tool_id: source_tool_id,
                            output: source_output_id,
                            to_tool_id: tool_id,
                            input: cloned_input_id.clone(),
                        }
                    })
            })
    }

    pub(crate) fn output_nodes_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = NodeIndex<GraphSize>> + '_> {
        self.assert_tool_exists(tool_id).ok()?;
        Some(
            self.graph
                .neighbors_directed(tool_id.into(), Direction::Outgoing),
        )
    }

    pub(crate) fn outputs_for_tool(
        &self,
        tool_id: ToolId,
    ) -> Option<impl Iterator<Item = &'_ ToolOutlet> + '_> {
        self.output_nodes_for_tool(tool_id).map(|node_ids| {
            node_ids.map(|node_id| match &self.graph[node_id] {
                NodeData::Output(outlet) => outlet,
                _ => panic!("Expected output node"),
            })
        })
    }

    pub(crate) fn get_output_node(
        &self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<NodeIndex<GraphSize>> {
        self.assert_tool_exists(tool_id).ok()?;
        self.graph
            .edges_directed(tool_id.into(), Direction::Outgoing)
            .find_map(|edge| {
                if let WireData::OutletConnection(edge_output_id) = edge.weight() {
                    if edge_output_id == output_id {
                        debug_assert!(matches!(self.graph[edge.target()], NodeData::Output(_)));
                        Some(edge.target())
                    } else {
                        None
                    }
                } else {
                    panic!("Expected outlet connection");
                }
            })
    }

    pub(crate) fn get_output(
        &self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<&ToolOutlet> {
        self.get_output_node(tool_id, output_id)
            .map(|node_id| match &self.graph[node_id] {
                NodeData::Output(outlet) => outlet,
                _ => panic!("Expected output node"),
            })
    }

    pub(crate) fn add_output_for_tool(
        &mut self,
        tool_id: ToolId,
        outlet_reference: ToolOutletReference,
        extras: Extras,
    ) -> Result<ToolOutlet, Error> {
        self.assert_tool_exists(tool_id)?;
        let outlet = ToolOutlet {
            outlet: outlet_reference,
            tool_id,
            extras,
        };
        let output_node = self.graph.add_node(NodeData::Output(outlet.clone()));
        self.graph.add_edge(
            tool_id.into(),
            output_node,
            WireData::OutletConnection(outlet.outlet.tool_outlet_id.clone()),
        );
        Ok(outlet)
    }

    pub(crate) fn remove_output_for_tool(
        &mut self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<ToolOutlet> {
        self.assert_tool_exists(tool_id).ok()?;
        let output_node = self.get_output_node(tool_id, output_id)?;
        match self.graph.remove_node(output_node) {
            Some(NodeData::Output(outlet)) => Some(outlet),
            None => panic!("Node was removed after lookup"),
            _ => panic!("Expected output node"),
        }
    }

    pub(crate) fn get_output_extras(
        &self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<&Extras> {
        self.get_output_node(tool_id, output_id)
            .map(|node_id| match &self.graph[node_id] {
                NodeData::Output(outlet) => &outlet.extras,
                _ => panic!("Expected output node"),
            })
    }

    pub(crate) fn get_output_extras_mut(
        &mut self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<&mut Extras> {
        self.get_output_node(tool_id, output_id)
            .map(move |node_id| match &mut self.graph[node_id] {
                NodeData::Output(outlet) => &mut outlet.extras,
                _ => panic!("Expected output node"),
            })
    }

    pub(crate) fn wires_for_output(
        &self,
        tool_id: ToolId,
        output_id: &ToolOutletId,
    ) -> Option<impl Iterator<Item = WireRef> + '_> {
        let cloned_output_id = output_id.clone();
        self.get_output_node(tool_id, output_id)
            .map(move |output_node| {
                self.graph
                    .edges_directed(output_node, Direction::Outgoing)
                    .map(move |edge| {
                        let target_node = edge.target();
                        let (target_tool_id, target_input_id) = match &self.graph[target_node] {
                            NodeData::Input(input_outlet) => (
                                input_outlet.tool_id,
                                input_outlet.outlet.tool_outlet_id.clone(),
                            ),
                            _ => panic!("Expected input node"),
                        };
                        WireRef {
                            id: edge.id().index() as GraphSize,
                            from_tool_id: tool_id,
                            output: cloned_output_id.clone(),
                            to_tool_id: target_tool_id,
                            input: target_input_id,
                        }
                    })
            })
    }

    pub(crate) fn add_wire(
        &mut self,
        from_tool_id: ToolId,
        output: ToolOutletId,
        to_tool_id: ToolId,
        input: ToolOutletId,
        extras: Extras,
    ) -> Result<WireRef, Error> {
        let from_node = self
            .get_output_node(from_tool_id, &output)
            .ok_or_else(|| anyhow!("Output {} not found for tool {:?}", &output, from_tool_id))?;
        let to_node = self
            .get_input_node(to_tool_id, &input)
            .ok_or_else(|| anyhow!("Input {} not found for tool {:?}", &input, to_tool_id))?;
        let edge = self
            .graph
            .add_edge(from_node, to_node, WireData::ToolConnection(extras));
        Ok(WireRef {
            id: edge.index() as GraphSize,
            from_tool_id,
            output,
            to_tool_id,
            input,
        })
    }

    pub(crate) fn remove_wire(&mut self, wire_id: WireRef) -> Option<()> {
        self.graph.remove_edge(wire_id.into()).map(|_| ())
    }

    pub(crate) fn get_wire_extras(&self, wire: WireRef) -> Option<&Extras> {
        self.graph
            .edge_weight(wire.into())
            .and_then(|edge| match edge {
                WireData::ToolConnection(extras) => Some(extras),
                _ => panic!("Expected tool connection"),
            })
    }

    pub(crate) fn get_wire_extras_mut(&mut self, wire: WireRef) -> Option<&mut Extras> {
        self.graph
            .edge_weight_mut(wire.into())
            .and_then(|edge| match edge {
                WireData::ToolConnection(extras) => Some(extras),
                _ => panic!("Expected tool connection"),
            })
    }
}
