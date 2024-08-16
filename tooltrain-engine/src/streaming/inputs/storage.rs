use std::collections::BTreeMap;

use tokio_stream::Stream;

use crate::{
    bindings::streaming_inputs::{ListChange, TreeChange},
    streaming::storage::DataStreamResourceChange,
};

use super::change_streams::{InputChangeStream, ReplacementChangeFromDataStreamSnapshot};

pub(super) struct InputStreamsStorage<T: Clone + ReplacementChangeFromDataStreamSnapshot>(
    BTreeMap<u32, InputChangeStream<T>>,
);

impl<T: Clone + ReplacementChangeFromDataStreamSnapshot> Default for InputStreamsStorage<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: Clone + ReplacementChangeFromDataStreamSnapshot> InputStreamsStorage<T> {
    pub(super) fn add_stream<S, R>(
        &mut self,
        input_id: u32,
        stream_changes: S,
        resource_changes: R,
    ) -> u32
    where
        S: Stream<Item = T>,
        S: Send,
        S: 'static,
        R: Stream<Item = DataStreamResourceChange>,
        R: Send,
        R: 'static,
    {
        let next_id = self.0.last_key_value().map(|(k, _)| k + 1).unwrap_or(0);
        self.0.insert(
            next_id,
            InputChangeStream::new(
                input_id,
                Box::pin(stream_changes),
                Box::pin(resource_changes),
            ),
        );
        next_id
    }

    pub(super) fn get_mut(&mut self, id: u32) -> Option<&mut InputChangeStream<T>> {
        self.0.get_mut(&id)
    }

    pub(super) fn remove(&mut self, id: u32) -> bool {
        self.0.remove(&id).is_some()
    }
}

#[derive(Default)]
pub(crate) struct InputStreams {
    pub(super) value_streams: InputStreamsStorage<Option<Vec<u8>>>,
    pub(super) list_streams: InputStreamsStorage<ListChange>,
    pub(super) tree_streams: InputStreamsStorage<TreeChange>,
}
