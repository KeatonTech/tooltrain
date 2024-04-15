use std::pin::Pin;

use anyhow::{anyhow, Error};
use commander_data::{CommanderCoder, CommanderDataType};
use futures::FutureExt;
use tokio_stream::{Stream, StreamExt};

use crate::{
    bindings::streaming_inputs::{ListChange, TreeChange},
    datastream::DataStreamSnapshot,
    streaming::{storage::DataStreamResourceChange, DataStreamStorage},
};

pub(super) trait ReplacementChangeFromDataStreamSnapshot: Sized {
    fn replace_from_snapshot(
        snapshot: &DataStreamSnapshot,
        data_type: &CommanderDataType,
    ) -> Result<Self, Error>;
}

pub(super) struct InputChangeStream<T: Clone + ReplacementChangeFromDataStreamSnapshot> {
    pub(super) input_id: u32,
    stream_changes: Pin<Box<dyn Stream<Item = T> + Send>>,
    resource_changes: Pin<Box<dyn Stream<Item = DataStreamResourceChange> + Send>>,
}
impl<T: Clone + ReplacementChangeFromDataStreamSnapshot> InputChangeStream<T> {
    pub(super) fn new(
        input_id: u32,
        stream_changes: Pin<Box<dyn Stream<Item = T> + Send>>,
        resource_changes: Pin<Box<dyn Stream<Item = DataStreamResourceChange> + Send>>,
    ) -> Self {
        Self {
            input_id,
            stream_changes,
            resource_changes,
        }
    }

    pub fn poll_change(&mut self) -> Result<Option<T>, Error> {
        Ok(self.stream_changes.next().now_or_never().flatten())
    }

    pub async fn poll_change_blocking(&mut self, storage: DataStreamStorage) -> Result<T, Error> {
        tokio::select! {
            stream_change = self.stream_changes.next() => {
                stream_change.ok_or(anyhow!("Stream ended unexpectedly"))
            }
            resource_change_optional = self.resource_changes.next() => {
                let resource_change = resource_change_optional.ok_or(anyhow!("Resource stream ended unexpectedly"))?;
                assert!(resource_change.is_data_stream_changed());
                assert_eq!(resource_change.unwrap_data_stream_changed(), self.input_id);
                let input = storage.get(self.input_id)?;
                let snapshot = input.stream.read().snapshot();
                T::replace_from_snapshot(&snapshot,&input.metadata.data_type)
            }
        }
    }
}

impl ReplacementChangeFromDataStreamSnapshot for Option<Vec<u8>> {
    fn replace_from_snapshot(
        snapshot: &DataStreamSnapshot,
        data_type: &CommanderDataType,
    ) -> Result<Self, Error> {
        match snapshot {
            DataStreamSnapshot::Value(maybe_value) => maybe_value.as_deref()
                .map(|value| data_type.encode(value.clone()))
                .transpose(),
            _ => Err(anyhow!(
                "Value Change can only be created from Value snapshot"
            )),
        }
    }
}

impl ReplacementChangeFromDataStreamSnapshot for ListChange {
    fn replace_from_snapshot(
        snapshot: &DataStreamSnapshot,
        data_type: &CommanderDataType,
    ) -> Result<Self, Error> {
        match snapshot {
            DataStreamSnapshot::List(l) => Ok(ListChange::Replace(
                l.iter()
                    .map(|v| data_type.encode((**v).clone()))
                    .collect::<Result<Vec<_>, Error>>()?,
            )),
            _ => Err(anyhow!("ListChange can only be created from List snapshot")),
        }
    }
}

impl ReplacementChangeFromDataStreamSnapshot for TreeChange {
    fn replace_from_snapshot(
        snapshot: &DataStreamSnapshot,
        _: &CommanderDataType,
    ) -> Result<Self, Error> {
        match snapshot {
            DataStreamSnapshot::Tree(t) => Ok(TreeChange::Replace(
                t.iter()
                    .map(|stream_node| (*stream_node.value).clone())
                    .collect::<Vec<_>>(),
            )),
            _ => Err(anyhow!("TreeChange can only be created from Tree snapshot")),
        }
    }
}
