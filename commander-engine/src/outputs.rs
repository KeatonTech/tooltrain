use std::collections::BTreeMap;

use crate::{
    bindings::{DataType, Value},
    datastream::{DataStream, DataStreamSnapshot, ListStream, TreeStream, ValueStream},
    Column,
};
use anyhow::Error;
use tokio::sync::broadcast::{channel, Sender};

pub(crate) type OutputId = u32;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum SpecChange {
    Added(OutputId),
    Updated(OutputId),
    Removed(OutputId),
}

#[derive(Clone, Debug)]
pub enum OutputDataType {
    List(Vec<Column>),
    Tree(DataType),
    Value(DataType),
}

#[derive(Debug)]
pub struct OutputSpec {
    pub name: String,
    pub description: String,
    pub data_type: OutputDataType,
    pub stream: DataStream,
}

pub(crate) struct Outputs {
    pub(crate) state: BTreeMap<OutputId, OutputSpec>,
    pub(crate) updates: Sender<SpecChange>,
}

impl Default for Outputs {
    fn default() -> Self {
        let (updates, _) = channel(128);
        Outputs {
            state: BTreeMap::new(),
            updates,
        }
    }
}

impl Outputs {
    fn add_output(
        &mut self,
        name: String,
        description: String,
        data_type: OutputDataType,
        stream: DataStream,
    ) -> Result<OutputId, Error> {
        let next_index = self.state.last_key_value().map(|(k, _)| k + 1).unwrap_or(0);
        self.state.insert(
            next_index,
            OutputSpec {
                name,
                description,
                data_type,
                stream,
            },
        );
        let _ = self.updates.send(SpecChange::Added(next_index));
        Ok(next_index)
    }

    pub(crate) fn add_value_output(
        &mut self,
        name: String,
        description: String,
        data_type: DataType,
        value: Option<Value>,
    ) -> Result<OutputId, Error> {
        self.add_output(
            name,
            description,
            OutputDataType::Value(data_type),
            DataStream::Value(ValueStream::new(value)),
        )
    }

    pub(crate) fn add_list_output(
        &mut self,
        name: String,
        description: String,
        columns: Vec<Column>,
    ) -> Result<OutputId, Error> {
        self.add_output(
            name,
            description,
            OutputDataType::List(columns),
            DataStream::List(ListStream::new()),
        )
    }

    pub(crate) fn add_tree_output(
        &mut self,
        name: String,
        description: String,
        data_type: DataType,
    ) -> Result<OutputId, Error> {
        self.add_output(
            name,
            description,
            OutputDataType::Tree(data_type),
            DataStream::Tree(TreeStream::new()),
        )
    }

    pub(crate) fn get_output(&self, id: OutputId) -> Option<&OutputSpec> {
        self.state.get(&id)
    }

    pub(crate) fn get_output_mut(&mut self, id: OutputId) -> Option<&mut OutputSpec> {
        self.state.get_mut(&id)
    }

    pub(crate) fn remove_output(&mut self, id: OutputId) -> Result<bool, Error> {
        if let Some(output) = self.state.remove(&id) {
            output.stream.destroy()?;
            let _ = self.updates.send(SpecChange::Removed(id));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn snapshot(&self) -> BTreeMap<OutputId, DataStreamSnapshot> {
        self.state
            .iter()
            .map(|(id, spec)| (*id, spec.stream.snapshot()))
            .collect()
    }
}
