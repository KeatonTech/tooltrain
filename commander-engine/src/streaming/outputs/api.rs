use std::{collections::BTreeMap, sync::Arc};

use crate::{
    datastream::{DataStreamSnapshot, ListChange, TreeChange, TreeStreamNode, ValueChange},
    streaming::storage::{
        DataStreamMetadata, DataStreamResourceChange, DataStreamStorage, DataStreamType, ResourceId,
    },
};
use anyhow::Error;
use commander_data::CommanderValue;
use tokio::sync::broadcast::Receiver;
use tokio_stream::{once, wrappers::BroadcastStream, Stream, StreamExt};

fn make_broadcast_stream<T: Clone + Send + 'static>(
    broadcast_receiver: Receiver<T>,
) -> impl Stream<Item = T> {
    BroadcastStream::new(broadcast_receiver).map_while(Result::ok)
}

#[derive(Clone, Debug)]
pub struct ValueOutputHandle {
    pub metadata: DataStreamMetadata,
}

impl ValueOutputHandle {
    pub fn load<'a>(&self, from_storage: Outputs<'a>) -> ValueOutputRef<'a> {
        ValueOutputRef {
            storage: from_storage.0,
            id: self.metadata.id,
        }
    }
}

#[derive(Debug)]
pub struct ValueOutputRef<'a> {
    storage: &'a DataStreamStorage,
    id: ResourceId,
}

impl<'a> ValueOutputRef<'a> {
    pub fn metadata(&self) -> DataStreamMetadata {
        self.storage.get(self.id).unwrap().metadata.clone()
    }

    pub fn snapshot(&self) -> Result<Option<Arc<CommanderValue>>, Error> {
        Ok(self
            .storage
            .get(self.id)?
            .stream
            .read()
            .try_get_value()?
            .snapshot())
    }

    pub fn updates(&self) -> Result<impl Stream<Item = ValueChange>, Error> {
        Ok(make_broadcast_stream(
            self.storage
                .get(self.id)?
                .stream
                .read()
                .try_get_value()?
                .subscribe(),
        ))
    }

    pub fn values(&self) -> Result<impl Stream<Item = Option<Arc<CommanderValue>>> + '_, Error> {
        Ok(once(self.snapshot()?).chain(self.updates()?.map_while(|_| self.snapshot().ok())))
    }
}

#[derive(Clone, Debug)]
pub struct ListOutputHandle {
    pub metadata: DataStreamMetadata,
}

impl ListOutputHandle {
    pub fn load<'a>(&self, from_storage: Outputs<'a>) -> ListOutputRef<'a> {
        ListOutputRef {
            storage: from_storage.0,
            id: self.metadata.id,
        }
    }
}

#[derive(Debug)]
pub struct ListOutputRef<'a> {
    storage: &'a DataStreamStorage,
    id: ResourceId,
}

impl<'a> ListOutputRef<'a> {
    pub fn metadata(&self) -> DataStreamMetadata {
        self.storage.get(self.id).unwrap().metadata.clone()
    }

    pub fn snapshot(&self) -> Result<Vec<Arc<CommanderValue>>, Error> {
        Ok(self
            .storage
            .get(self.id)?
            .stream
            .read()
            .try_get_list()?
            .snapshot())
    }

    pub fn updates(&self) -> Result<impl Stream<Item = ListChange>, Error> {
        Ok(make_broadcast_stream(
            self.storage
                .get(self.id)?
                .stream
                .read()
                .try_get_list()?
                .subscribe(),
        ))
    }

    pub fn values(&self) -> Result<impl Stream<Item = Vec<Arc<CommanderValue>>> + '_, Error> {
        Ok(once(self.snapshot()?).chain(self.updates()?.map_while(|_| self.snapshot().ok())))
    }

    pub fn load_more(&self, limit: u32) -> Result<bool, Error> {
        self.storage
            .get(self.id)?
            .stream
            .write()
            .try_get_list_mut()?
            .request_page(limit)
    }
}

#[derive(Clone, Debug)]
pub struct TreeOutputHandle {
    pub metadata: DataStreamMetadata,
}

impl TreeOutputHandle {
    pub fn load<'a>(&self, from_storage: Outputs<'a>) -> TreeOutputRef<'a> {
        TreeOutputRef {
            storage: from_storage.0,
            id: self.metadata.id,
        }
    }
}

#[derive(Debug)]
pub struct TreeOutputRef<'a> {
    storage: &'a DataStreamStorage,
    id: ResourceId,
}

impl<'a> TreeOutputRef<'a> {
    pub fn metadata(&self) -> DataStreamMetadata {
        self.storage.get(self.id).unwrap().metadata.clone()
    }

    pub fn snapshot(&self) -> Result<Vec<TreeStreamNode>, Error> {
        Ok(self
            .storage
            .get(self.id)?
            .stream
            .read()
            .try_get_tree()?
            .snapshot())
    }

    pub fn updates(&self) -> Result<impl Stream<Item = TreeChange>, Error> {
        Ok(make_broadcast_stream(
            self.storage
                .get(self.id)?
                .stream
                .read()
                .try_get_tree()?
                .subscribe(),
        ))
    }

    pub fn values(&self) -> Result<impl Stream<Item = Vec<TreeStreamNode>> + '_, Error> {
        Ok(once(self.snapshot()?).chain(self.updates()?.map_while(|_| self.snapshot().ok())))
    }

    pub fn request_children(&self, parent: String) -> Result<bool, Error> {
        self.storage
            .get(self.id)?
            .stream
            .write()
            .try_get_tree_mut()?
            .request_children(parent)
    }
}

#[derive(Clone, Debug)]
pub enum OutputHandle {
    List(ListOutputHandle),
    Tree(TreeOutputHandle),
    Value(ValueOutputHandle),
}

impl OutputHandle {
    pub fn metadata(&self) -> &DataStreamMetadata {
        match self {
            OutputHandle::List(l) => &l.metadata,
            OutputHandle::Tree(t) => &t.metadata,
            OutputHandle::Value(v) => &v.metadata,
        }
    }

    fn from_metadata(metadata: DataStreamMetadata) -> Self {
        match metadata.data_stream_type {
            DataStreamType::Value => OutputHandle::Value(ValueOutputHandle { metadata }),
            DataStreamType::List => OutputHandle::List(ListOutputHandle { metadata }),
            DataStreamType::Tree => OutputHandle::Tree(TreeOutputHandle { metadata }),
        }
    }
}

pub struct Outputs<'a>(pub(crate) &'a DataStreamStorage);

#[derive(Debug)]
pub enum OutputChange {
    Added(OutputHandle),
    Removed(ResourceId),
}

impl<'a> Outputs<'a> {
    pub fn updates(&self) -> impl Stream<Item = OutputChange> + '_ {
        BroadcastStream::from(self.0.changes())
            .map_while(|result| result.ok())
            .map(|internal_change| match internal_change {
                DataStreamResourceChange::Added(metadata) => {
                    OutputChange::Added(OutputHandle::from_metadata(metadata))
                }
                DataStreamResourceChange::Removed(id) => OutputChange::Removed(id),
            })
    }

    pub fn values(&self) -> impl Stream<Item = Vec<OutputHandle>> + '_ {
        once(self.snapshot_metadata()).chain(self.updates().map(|_| self.snapshot_metadata()))
    }

    pub fn snapshot_metadata(&self) -> Vec<OutputHandle> {
        self.0
            .state()
            .values()
            .map(|state| OutputHandle::from_metadata(state.metadata.clone()))
            .collect()
    }

    pub fn snapshot_output_values(&self) -> BTreeMap<ResourceId, DataStreamSnapshot> {
        self.0
            .state()
            .iter()
            .map(|(id, spec)| (*id, spec.stream.read().snapshot()))
            .collect()
    }
}
