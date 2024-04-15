use std::sync::Arc;
use std::{collections::BTreeMap};

use crate::datastream::DataStream;
use crate::streaming::inputs::storage::InputStreams;

use anyhow::{anyhow, Error};
use cap_std::fs::Dir;

use commander_data::CommanderDataType;
use derive_more::{IsVariant, TryInto, Unwrap};
use parking_lot::{
    MappedRwLockReadGuard, RwLock, RwLockReadGuard,
};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use wasmtime::component::*;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView};

use super::outputs::storage::OutputRequestStreams;

pub type ResourceId = u32;

#[derive(Clone, Debug, TryInto, IsVariant, Unwrap)]
pub enum DataStreamResourceChange {
    Added(DataStreamMetadata),
    Removed(ResourceId),
    DataStreamChanged(ResourceId),
}

#[derive(Clone, Copy, Debug)]
pub enum DataStreamType {
    Value,
    List,
    Tree,
}

#[derive(Clone, Debug)]
pub struct DataStreamMetadata {
    pub id: ResourceId,
    pub name: String,
    pub description: String,
    pub data_type: CommanderDataType,
    pub data_stream_type: DataStreamType,
}

#[derive(Debug)]
pub(crate) struct DataStreamResource {
    pub metadata: DataStreamMetadata,
    pub stream: Arc<RwLock<DataStream>>,
}

#[derive(Debug)]
pub(crate) struct DataStreamStorageInternal {
    state: BTreeMap<ResourceId, DataStreamResource>,
    changes: Sender<DataStreamResourceChange>,
}

#[derive(Clone, Debug)]
pub(crate) struct DataStreamStorage(Arc<RwLock<DataStreamStorageInternal>>);

impl Default for DataStreamStorage {
    fn default() -> Self {
        let (changes, _) = channel(128);
        DataStreamStorage(Arc::new(RwLock::new(DataStreamStorageInternal {
            state: BTreeMap::new(),
            changes,
        })))
    }
}

impl DataStreamStorage {
    pub(crate) fn add(
        &self,
        name: String,
        description: String,
        data_type: CommanderDataType,
        stream: Arc<RwLock<DataStream>>,
    ) -> Result<ResourceId, Error> {
        let mut writer = self.0.write();
        let next_index = writer
            .state
            .last_key_value()
            .map(|(k, _)| k + 1)
            .unwrap_or(0);
        let metadata = DataStreamMetadata {
            id: next_index,
            name,
            description,
            data_type,
            data_stream_type: match *stream.read() {
                DataStream::Value(_) => DataStreamType::Value,
                DataStream::List(_) => DataStreamType::List,
                DataStream::Tree(_) => DataStreamType::Tree,
            },
        };
        writer.state.insert(
            next_index,
            DataStreamResource {
                metadata: metadata.clone(),
                stream
            },
        );
        let _ = writer
            .changes
            .send(DataStreamResourceChange::Added(metadata));
        Ok(next_index)
    }

    pub(crate) fn remove(&mut self, id: ResourceId) -> Result<bool, Error> {
        let mut writer = self.0.write();
        if let Some(output) = writer.state.remove(&id) {
            let stream = output.stream;
            if let Some(inner_stream) = Arc::into_inner(stream) {
                inner_stream.into_inner().destroy()?;
            }
            let _ = writer.changes.send(DataStreamResourceChange::Removed(id));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn get(
        &self,
        id: ResourceId,
    ) -> Result<MappedRwLockReadGuard<'_, DataStreamResource>, Error> {
        RwLockReadGuard::try_map(self.0.read(), |internal| internal.state.get(&id))
            .map_err(|_| anyhow!("Output does not exist"))
    }

    pub(crate) fn change_data_stream(&self, id: ResourceId, new_stream: Arc<RwLock<DataStream>>) -> Result<(), Error> {
        let mut writer = self.0.write();
        writer.state.get_mut(&id).ok_or_else(|| anyhow!("Stream does not exist"))?.stream = new_stream;
        writer.changes.send(DataStreamResourceChange::DataStreamChanged(id))?;
        Ok(())
    }

    pub(crate) fn changes(&self) -> Receiver<DataStreamResourceChange> {
        self.0.read().changes.subscribe()
    }

    pub(crate) fn state(
        &self,
    ) -> MappedRwLockReadGuard<'_, BTreeMap<ResourceId, DataStreamResource>> {
        RwLockReadGuard::map(self.0.read(), |inner| &inner.state)
    }
}

pub(crate) struct WasmStorage {
    table: ResourceTable,
    ctx: WasiCtx,
    http_ctx: WasiHttpCtx,
    pub(crate) outputs: DataStreamStorage,
    pub(crate) output_request_streams: OutputRequestStreams,
    pub(crate) inputs: DataStreamStorage,
    pub(crate) input_streams: InputStreams
}

impl WasiView for WasmStorage {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasiHttpView for WasmStorage {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WasmStorage {
    pub(crate) fn new() -> Self {
        Self {
            table: ResourceTable::new(),
            ctx: WasiCtxBuilder::new()
                .preopened_dir(
                    Dir::from_std_file(std::fs::File::open("/").unwrap()),
                    DirPerms::READ,
                    FilePerms::READ,
                    "/",
                )
                .inherit_stdio()
                .inherit_stderr()
                .build(),
            http_ctx: WasiHttpCtx,
            outputs: Default::default(),
            output_request_streams: Default::default(),
            inputs: Default::default(),
            input_streams: Default::default(),
        }
    }
}
