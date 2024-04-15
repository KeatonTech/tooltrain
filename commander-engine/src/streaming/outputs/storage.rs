use std::{collections::BTreeMap, pin::Pin};

use anyhow::{anyhow, Error};
use futures::FutureExt;
use tokio_stream::{Stream, StreamExt};

use crate::bindings::streaming_outputs::{ListOutputRequest, TreeOutputRequest};

pub(super) struct OutputRequestStream<T>(Pin<Box<dyn Stream<Item = T> + Send>>);

impl<T> OutputRequestStream<T> {
    pub(super) fn poll_request(&mut self) -> Result<Option<T>, Error> {
        self.poll_request_blocking().now_or_never().transpose()
    }

    pub(super) async fn poll_request_blocking(&mut self) -> Result<T, Error> {
        self.0
            .next()
            .await
            .ok_or_else(|| anyhow!("Unexpected end of stream"))
    }
}

pub(super) struct OutputRequestStreamStorage<T>(BTreeMap<u32, OutputRequestStream<T>>);

impl<T> Default for OutputRequestStreamStorage<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T> OutputRequestStreamStorage<T> {
    pub(super) fn add_stream<S>(&mut self, stream: S) -> u32
    where
        S: Stream<Item = T>,
        S: Send,
        S: 'static,
    {
        let next_id = self.0.last_key_value().map(|(k, _)| k + 1).unwrap_or(0);
        self.0
            .insert(next_id, OutputRequestStream(Box::pin(stream)));
        next_id
    }

    pub(super) fn get_mut(&mut self, id: u32) -> Option<&mut OutputRequestStream<T>> {
        self.0.get_mut(&id)
    }

    pub(super) fn remove(&mut self, id: u32) -> bool {
        self.0.remove(&id).is_some()
    }
}

#[derive(Default)]
pub(crate) struct OutputRequestStreams {
    pub(super) list_request_streams: OutputRequestStreamStorage<ListOutputRequest>,
    pub(super) tree_request_streams: OutputRequestStreamStorage<TreeOutputRequest>,
}
