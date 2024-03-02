use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use super::structure::OutputChangeInternal;
use crate::{
    bindings::Value,
    datastream::{
        DataStreamSnapshot, ListChange, ListStream, TreeChange, TreeStream, TreeStreamNode,
        ValueChange, ValueStream,
    },
    OutputDataType, OutputId, OutputMetadata, Outputs,
};
use anyhow::{anyhow, Error};
use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};
use tokio::sync::broadcast::Receiver;
use tokio_stream::{once, wrappers::BroadcastStream, Stream, StreamExt};

fn make_broadcast_stream<T: Clone + Send + 'static>(
    broadcast_receiver: Receiver<T>,
) -> impl Stream<Item = T> {
    BroadcastStream::new(broadcast_receiver).map_while(Result::ok)
}

impl OutputMetadata {
    fn to_handle(&self, outputs: Outputs) -> OutputHandle {
        match self.data_type {
            OutputDataType::List(_) => OutputHandle::List(ListOutputHandle {
                outputs,
                id: self.id,
            }),
            OutputDataType::Tree(_) => OutputHandle::Tree(TreeOutputHandle {
                outputs,
                id: self.id,
            }),
            OutputDataType::Value(_) => OutputHandle::Value(ValueOutputHandle {
                outputs,
                id: self.id,
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub enum OutputChange {
    Added(OutputHandle),
    Removed(OutputId),
}

#[derive(Clone, Debug)]
pub struct ValueOutputHandle {
    outputs: Outputs,
    id: OutputId,
}

impl ValueOutputHandle {
    fn value_stream(&self) -> Result<impl Deref<Target = ValueStream> + '_, Error> {
        MappedRwLockReadGuard::try_map(self.outputs.get_output(self.id)?, |output| {
            output.stream.try_get_value().ok()
        })
        .map_err(|_| anyhow!("Output is not a ValueStream"))
    }

    pub fn metadata(&self) -> OutputMetadata {
        self.outputs.get_output(self.id).unwrap().metadata.clone()
    }

    pub fn snapshot(&self) -> Result<Option<Arc<Value>>, Error> {
        Ok(self.value_stream()?.snapshot())
    }

    pub fn updates(&self) -> Result<impl Stream<Item = ValueChange>, Error> {
        Ok(make_broadcast_stream(self.value_stream()?.subscribe()))
    }

    pub fn values(&self) -> Result<impl Stream<Item = Option<Arc<Value>>> + '_, Error> {
        Ok(once(self.snapshot()?).chain(self.updates()?.map_while(|_| self.snapshot().ok())))
    }
}

#[derive(Clone, Debug)]
pub struct ListOutputHandle {
    outputs: Outputs,
    id: OutputId,
}

impl ListOutputHandle {
    fn list_stream(&self) -> Result<impl Deref<Target = ListStream> + '_, Error> {
        MappedRwLockReadGuard::try_map(self.outputs.get_output(self.id)?, |output| {
            output.stream.try_get_list().ok()
        })
        .map_err(|_| anyhow!("Output is not a ListStream"))
    }

    fn list_stream_mut(&self) -> Result<impl DerefMut<Target = ListStream> + '_, Error> {
        MappedRwLockWriteGuard::try_map(self.outputs.get_output_mut(self.id)?, |output| {
            output.stream.try_get_list_mut().ok()
        })
        .map_err(|_| anyhow!("Output is not a ListStream"))
    }

    pub fn metadata(&self) -> OutputMetadata {
        self.outputs.get_output(self.id).unwrap().metadata.clone()
    }

    pub fn snapshot(&self) -> Result<Vec<Arc<Value>>, Error> {
        Ok(self.list_stream()?.snapshot())
    }

    pub fn updates(&self) -> Result<impl Stream<Item = ListChange>, Error> {
        Ok(make_broadcast_stream(self.list_stream()?.subscribe()))
    }

    pub fn values(&self) -> Result<impl Stream<Item = Vec<Arc<Value>>> + '_, Error> {
        Ok(once(self.snapshot()?).chain(self.updates()?.map_while(|_| self.snapshot().ok())))
    }

    pub fn load_more(&self, limit: u32) -> Result<bool, Error> {
        self.list_stream_mut()?.request_page(limit)
    }
}

#[derive(Clone, Debug)]
pub struct TreeOutputHandle {
    outputs: Outputs,
    id: OutputId,
}

impl TreeOutputHandle {
    fn tree_stream(&self) -> Result<impl Deref<Target = TreeStream> + '_, Error> {
        MappedRwLockReadGuard::try_map(self.outputs.get_output(self.id)?, |output| {
            output.stream.try_get_tree().ok()
        })
        .map_err(|_| anyhow!("Output is not a TreeStream"))
    }

    fn tree_stream_mut(&self) -> Result<impl DerefMut<Target = TreeStream> + '_, Error> {
        MappedRwLockWriteGuard::try_map(self.outputs.get_output_mut(self.id)?, |output| {
            output.stream.try_get_tree_mut().ok()
        })
        .map_err(|_| anyhow!("Output is not a TreeStream"))
    }

    pub fn metadata(&self) -> OutputMetadata {
        self.outputs.get_output(self.id).unwrap().metadata.clone()
    }

    pub fn snapshot(&self) -> Result<Vec<TreeStreamNode>, Error> {
        Ok(self.tree_stream()?.snapshot())
    }

    pub fn updates(&self) -> Result<impl Stream<Item = TreeChange>, Error> {
        Ok(make_broadcast_stream(self.tree_stream()?.subscribe()))
    }

    pub fn values(&self) -> Result<impl Stream<Item = Vec<TreeStreamNode>> + '_, Error> {
        Ok(once(self.snapshot()?).chain(self.updates()?.map_while(|_| self.snapshot().ok())))
    }

    pub fn request_children(&self, parent: String) -> Result<bool, Error> {
        self.tree_stream_mut()?.request_children(parent)
    }
}

#[derive(Clone, Debug)]
pub enum OutputHandle {
    List(ListOutputHandle),
    Tree(TreeOutputHandle),
    Value(ValueOutputHandle),
}

impl OutputHandle {
    pub fn metadata(&self) -> OutputMetadata {
        match self {
            OutputHandle::List(l) => l.metadata(),
            OutputHandle::Tree(t) => t.metadata(),
            OutputHandle::Value(v) => v.metadata(),
        }
    }
}

impl Outputs {
    pub fn updates(&self) -> impl Stream<Item = OutputChange> + '_ {
        BroadcastStream::from(self.0.read().updates.subscribe())
            .map_while(|result| result.ok())
            .map(|internal_change| match internal_change {
                OutputChangeInternal::Added(metadata) => {
                    OutputChange::Added(metadata.to_handle(self.clone()))
                }
                OutputChangeInternal::Removed(id) => OutputChange::Removed(id),
            })
    }

    pub fn values(&self) -> impl Stream<Item = Vec<OutputHandle>> + '_ {
        once(self.snapshot_metadata()).chain(self.updates().map(|_| self.snapshot_metadata()))
    }

    pub fn snapshot_metadata(&self) -> Vec<OutputHandle> {
        self.0
            .read()
            .state
            .values()
            .map(|state| state.metadata.to_handle(self.clone()))
            .collect()
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
