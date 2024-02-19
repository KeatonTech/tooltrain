use crate::{
    bindings::{
        commander::base::{self, streams::HostOutputHandle, types::ValueEvent},
        DataType, OutputHandle, PluginImports, Value,
    },
    outputs::Outputs,
};

use async_trait::async_trait;
use cap_std::fs::Dir;
use parking_lot::RwLock;
use std::sync::Arc;

use wasmtime::{component::*, Error};
use wasmtime_wasi::preview2::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView};

pub(crate) struct WasmStorage {
    table: ResourceTable,
    ctx: WasiCtx,
    outputs: Arc<RwLock<Outputs>>,
}

impl WasiView for WasmStorage {
    fn table(&self) -> &ResourceTable {
        &self.table
    }

    fn table_mut(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.ctx
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
            outputs: Arc::new(RwLock::new(Outputs::default())),
        }
    }

    pub(crate) fn get_outputs(&self) -> Arc<RwLock<Outputs>> {
        self.outputs.clone()
    }
}

impl base::types::Host for WasmStorage {}
impl base::streams::Host for WasmStorage {}

impl base::streams::HostRunHandle for WasmStorage {
    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<base::streams::RunHandle>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl HostOutputHandle for WasmStorage {
    async fn send(
        &mut self,
        handle: Resource<OutputHandle>,
        event: ValueEvent,
    ) -> wasmtime::Result<(), Error> {
        self.outputs.write().update_output(handle.rep(), event)
    }

    async fn remove(&mut self, handle: Resource<OutputHandle>) -> wasmtime::Result<(), Error> {
        drop(handle);
        Ok(())
    }

    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<OutputHandle>,
    ) -> wasmtime::Result<(), Error> {
        Ok(())
    }
}

#[async_trait]
impl PluginImports for WasmStorage {
    async fn add_output(
        &mut self,
        name: String,
        description: String,
        data_type: DataType,
        value: Option<Value>,
    ) -> Result<Resource<OutputHandle>, Error> {
        Ok(Resource::new_own(self.outputs.write().add_output(
            name,
            description,
            data_type,
            value,
        )))
    }
}
