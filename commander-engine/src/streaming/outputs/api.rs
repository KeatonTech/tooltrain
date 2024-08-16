use std::{collections::BTreeMap, sync::Arc};

use crate::{
    datastream::{
        DataStream, DataStreamSnapshot, ListChange, TreeChange, TreeStreamNode, ValueChange,
    },
    streaming::storage::{
        DataStreamMetadata, DataStreamResourceChange, DataStreamStorage, DataStreamType, ResourceId,
    },
};
use anyhow::Error;
use commander_data::CommanderValue;
use parking_lot::RwLock;
use tokio::sync::broadcast::Receiver;
use tokio_stream::{once, wrappers::BroadcastStream, Stream, StreamExt};

fn make_broadcast_stream<T: Clone + Send + 'static>(
    broadcast_receiver: Receiver<T>,
) -> impl Stream<Item = T> {
    BroadcastStream::new(broadcast_receiver).map_while(Result::ok)
}

pub trait OutputRef {
    fn inner_data_stream(&self) -> Result<Arc<RwLock<DataStream>>, Error>;
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

    pub fn value(&self) -> Result<Option<Arc<CommanderValue>>, Error> {
        Ok(self
            .storage
            .get(self.id)?
            .stream
            .read()
            .try_get_value()?
            .snapshot())
    }

    pub fn updates_stream(&self) -> Result<impl Stream<Item = ValueChange>, Error> {
        Ok(make_broadcast_stream(
            self.storage
                .get(self.id)?
                .stream
                .read()
                .try_get_value()?
                .subscribe(),
        ))
    }

    pub fn value_stream(
        &self,
    ) -> Result<impl Stream<Item = Option<Arc<CommanderValue>>> + '_, Error> {
        Ok(once(self.value()?).chain(self.updates_stream()?.map_while(|_| self.value().ok())))
    }
}

impl OutputRef for ValueOutputRef<'_> {
    fn inner_data_stream(&self) -> Result<Arc<RwLock<DataStream>>, Error> {
        Ok(self.storage.get(self.id)?.stream.clone())
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

    pub fn value(&self) -> Result<Vec<Arc<CommanderValue>>, Error> {
        Ok(self
            .storage
            .get(self.id)?
            .stream
            .read()
            .try_get_list()?
            .snapshot())
    }

    pub fn updates_stream(&self) -> Result<impl Stream<Item = ListChange>, Error> {
        Ok(make_broadcast_stream(
            self.storage
                .get(self.id)?
                .stream
                .read()
                .try_get_list()?
                .subscribe(),
        ))
    }

    pub fn values_stream(
        &self,
    ) -> Result<impl Stream<Item = Vec<Arc<CommanderValue>>> + '_, Error> {
        Ok(once(self.value()?).chain(self.updates_stream()?.map_while(|_| self.value().ok())))
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

impl OutputRef for ListOutputRef<'_> {
    fn inner_data_stream(&self) -> Result<Arc<RwLock<DataStream>>, Error> {
        Ok(self.storage.get(self.id)?.stream.clone())
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

    pub fn value(&self) -> Result<Vec<TreeStreamNode>, Error> {
        Ok(self
            .storage
            .get(self.id)?
            .stream
            .read()
            .try_get_tree()?
            .snapshot())
    }

    pub fn updates_stream(&self) -> Result<impl Stream<Item = TreeChange>, Error> {
        Ok(make_broadcast_stream(
            self.storage
                .get(self.id)?
                .stream
                .read()
                .try_get_tree()?
                .subscribe(),
        ))
    }

    pub fn value_stream(&self) -> Result<impl Stream<Item = Vec<TreeStreamNode>> + '_, Error> {
        Ok(once(self.value()?).chain(self.updates_stream()?.map_while(|_| self.value().ok())))
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

impl OutputRef for TreeOutputRef<'_> {
    fn inner_data_stream(&self) -> Result<Arc<RwLock<DataStream>>, Error> {
        Ok(self.storage.get(self.id)?.stream.clone())
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
            .filter_map(|internal_change| match internal_change {
                DataStreamResourceChange::Added(metadata) => {
                    Some(OutputChange::Added(OutputHandle::from_metadata(metadata)))
                }
                DataStreamResourceChange::Removed(id) => Some(OutputChange::Removed(id)),
                DataStreamResourceChange::DataStreamChanged(_) => None,
            })
    }

    pub fn handles_stream(&self) -> impl Stream<Item = Vec<OutputHandle>> + '_ {
        once(self.handles()).chain(self.updates().map(|_| self.handles()))
    }

    pub fn handles(&self) -> Vec<OutputHandle> {
        self.0
            .state()
            .values()
            .map(|state| OutputHandle::from_metadata(state.metadata.clone()))
            .collect()
    }

    pub fn values(&self) -> BTreeMap<ResourceId, DataStreamSnapshot> {
        self.0
            .state()
            .iter()
            .map(|(id, spec)| (*id, spec.stream.read().snapshot()))
            .collect()
    }
}
