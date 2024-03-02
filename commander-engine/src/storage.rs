use crate::{
    Outputs,
};



use cap_std::fs::Dir;


use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use wasmtime::component::*;
use wasmtime_wasi::preview2::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView};

pub(crate) struct WasmStorage {
    table: ResourceTable,
    ctx: WasiCtx,
    http_ctx: WasiHttpCtx,
    pub(crate) outputs: Outputs,
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
            outputs: Outputs::default(),
        }
    }

    pub(crate) fn get_outputs(&self) -> Outputs {
        self.outputs.clone()
    }
}

impl crate::bindings::commander::base::types::Host for WasmStorage {}
impl crate::bindings::commander::base::outputs::Host for WasmStorage {}
