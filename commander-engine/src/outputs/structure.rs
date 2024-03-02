use std::{
    collections::BTreeMap,
    sync::Arc,
};

use crate::{
    bindings::{DataType, Value},
    datastream::{
        DataStream, ListStream, TreeStream, ValueStream,
    },
    Column,
};
use anyhow::{anyhow, Error};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use tokio::sync::broadcast::{channel, Sender};

pub type OutputId = u32;

#[derive(Clone, Debug)]
pub(super) enum OutputChangeInternal {
    Added(OutputMetadata),
    Removed(OutputId),
}

#[derive(Clone, Debug)]
pub enum OutputDataType {
    List(Vec<Column>),
    Tree(DataType),
    Value(DataType),
}

#[derive(Clone, Debug)]
pub struct OutputMetadata {
    pub id: OutputId,
    pub name: String,
    pub description: String,
    pub data_type: OutputDataType,
}

#[derive(Debug)]
pub(crate) struct OutputState {
    pub metadata: OutputMetadata,
    pub stream: DataStream,
}

#[derive(Clone, Debug)]
pub struct Outputs(pub(super) Arc<RwLock<OutputsInternal>>);

#[derive(Debug)]
pub(super) struct OutputsInternal {
    pub(super) state: BTreeMap<OutputId, OutputState>,
    pub(super) updates: Sender<OutputChangeInternal>,
}

impl Default for Outputs {
    fn default() -> Self {
        let (updates, _) = channel(128);
        Outputs(Arc::new(RwLock::new(OutputsInternal {
            state: BTreeMap::new(),
            updates,
        })))
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
        let mut writer = self.0.write();
        let next_index = writer
            .state
            .last_key_value()
            .map(|(k, _)| k + 1)
            .unwrap_or(0);
        let metadata = OutputMetadata {
            id: next_index,
            name,
            description,
            data_type,
        };
        writer.state.insert(
            next_index,
            OutputState {
                metadata: metadata.clone(),
                stream,
            },
        );
        let _ = writer.updates.send(OutputChangeInternal::Added(metadata));
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

    pub(crate) fn get_output(
        &self,
        id: OutputId,
    ) -> Result<MappedRwLockReadGuard<'_, OutputState>, Error> {
        RwLockReadGuard::try_map(self.0.read(), |internal| internal.state.get(&id))
            .map_err(|_| anyhow!("Output does not exist"))
    }

    pub(crate) fn get_output_mut(
        &self,
        id: OutputId,
    ) -> Result<MappedRwLockWriteGuard<'_, OutputState>, Error> {
        RwLockWriteGuard::try_map(self.0.write(), |internal| internal.state.get_mut(&id))
            .map_err(|_| anyhow!("Output does not exist"))
    }

    pub(crate) fn remove_output(&mut self, id: OutputId) -> Result<bool, Error> {
        let mut writer = self.0.write();
        if let Some(output) = writer.state.remove(&id) {
            output.stream.destroy()?;
            let _ = writer.updates.send(OutputChangeInternal::Removed(id));
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
