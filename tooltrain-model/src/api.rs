use crate::{
    events::ToolchainModelMutation, internal::TooltrainModelInternal, TooltrainModelMutator,
    TooltrainModelReadLock,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone, Debug, Default)]
pub struct TooltrainModel(pub(crate) Arc<RwLock<TooltrainModelInternal>>);

impl<'de> Deserialize<'de> for TooltrainModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(Arc::new(RwLock::new(
            TooltrainModelInternal::deserialize(deserializer)?,
        ))))
    }
}

impl TooltrainModel {
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns a duplicate of this model that is fully detatched from this model.
    /// Changes to this model will not be reflected in the returned model, and vice versa.
    pub async fn detach(&self) -> Self {
        TooltrainModel(Arc::new(RwLock::new((*self.0).read().await.clone())))
    }

    pub async fn read(&self) -> TooltrainModelReadLock<'_> {
        (*self.0).read().await.into()
    }

    pub async fn write(&self) -> TooltrainModelMutator<'_> {
        TooltrainModelMutator::new(self).await
    }

    pub async fn subscribe(&self) -> BroadcastStream<ToolchainModelMutation> {
        BroadcastStream::new(self.0.read().await.mutations.subscribe())
    }
}
