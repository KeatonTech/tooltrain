use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    bindings::{DataType, Value},
    datastream::{
        DataStream, DataStreamSnapshot, ListChange, ListStream, TreeChange, TreeStream, TreeStreamNode, ValueChange, ValueStream
    },
    Column,
};
use anyhow::{anyhow, Error};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::sync::broadcast::{channel, Sender};
use tokio_stream::{once, wrappers::BroadcastStream, Stream, StreamExt};

pub(crate) type OutputId = u32;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum OutputChange {
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

pub struct OutputRef {
    outputs: Outputs,
    id: OutputId,
}

#[derive(Clone, Debug)]
pub(crate) struct Outputs(Arc<RwLock<OutputsInternal>>);

#[derive(Debug)]
struct OutputsInternal {
    state: BTreeMap<OutputId, OutputState>,
    updates: Sender<OutputChange>,
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
        writer.state.insert(
            next_index,
            OutputState {
                metadata: OutputMetadata {
                    id: next_index,
                    name,
                    description,
                    data_type,
                },
                stream,
            },
        );
        let _ = writer.updates.send(OutputChange::Added(next_index));
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
    ) -> Result<impl Deref<Target = OutputState> + '_, Error> {
        RwLockReadGuard::try_map(self.0.read(), |internal| internal.state.get(&id))
            .map_err(|_| anyhow!("Output does not exist"))
    }

    pub(crate) fn get_output_mut(
        &self,
        id: OutputId,
    ) -> Result<impl DerefMut<Target = OutputState> + '_, Error> {
        RwLockWriteGuard::try_map(self.0.write(), |internal| internal.state.get_mut(&id))
            .map_err(|_| anyhow!("Output does not exist"))
    }

    pub(crate) fn remove_output(&mut self, id: OutputId) -> Result<bool, Error> {
        let mut writer = self.0.write();
        if let Some(output) = writer.state.remove(&id) {
            output.stream.destroy()?;
            let _ = writer.updates.send(OutputChange::Removed(id));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn metadata_stream(&self) -> impl Stream<Item = Vec<OutputMetadata>> + '_ {
        once(self.snapshot_metadata())
            .chain(self.outputs_list_change_stream().map(|_| self.snapshot_metadata()))
    }

    pub fn snapshot_metadata(&self) -> Vec<OutputMetadata> {
        self.0
            .read()
            .state
            .values()
            .map(|s| &s.metadata)
            .cloned()
            .collect()
    }

    pub fn outputs_list_change_stream(&self) -> impl Stream<Item = OutputChange> {
        BroadcastStream::from(self.0.read().updates.subscribe()).map_while(|result| result.ok())
    }

    pub fn value_output_changes_stream(&self, id: OutputId) -> Result<impl Stream<Item = ValueChange>, Error> {
        Ok(
            BroadcastStream::from(self.get_output(id)?.stream.try_get_value()?.subscribe())
                .map_while(|result| result.ok()),
        )
    }

    pub fn value_output_stream(&self, id: OutputId) -> Result<impl Stream<Item = Option<Arc<Value>>> + '_, Error> {
        Ok(self.value_output_changes_stream(id)?.map_while(move |_| {
            Some(self.get_output(id).ok()?.stream.try_get_value().ok()?.snapshot())
        }))
    }

    pub fn list_output_changes_stream(&self, id: OutputId) -> Result<impl Stream<Item = ListChange>, Error> {
        Ok(
            BroadcastStream::from(self.get_output(id)?.stream.try_get_list()?.subscribe())
                .map_while(|result| result.ok()),
        )
    }

    pub fn list_output_stream(&self, id: OutputId) -> Result<impl Stream<Item = Vec<Arc<Value>>> + '_, Error> {
        Ok(self.list_output_changes_stream(id)?.map_while(move |_| {
            Some(self.get_output(id).ok()?.stream.try_get_list().ok()?.snapshot())
        }))
    }

    pub fn tree_output_changes_stream(&self, id: OutputId) -> Result<impl Stream<Item = TreeChange>, Error> {
        Ok(
            BroadcastStream::from(self.get_output(id)?.stream.try_get_tree()?.subscribe())
                .map_while(|result| result.ok()),
        )
    }

    pub fn tree_output_stream(&self, id: OutputId) -> Result<impl Stream<Item = Vec<TreeStreamNode>> + '_, Error> {
        Ok(self.tree_output_changes_stream(id)?.map_while(move |_| {
            Some(self.get_output(id).ok()?.stream.try_get_tree().ok()?.snapshot())
        }))
    }

    pub fn snapshot_output_values(&self) -> BTreeMap<OutputId, DataStreamSnapshot> {
        self.0
            .read()
            .state
            .iter()
            .map(|(id, spec)| (*id, spec.stream.snapshot()))
            .collect()
    }
}
