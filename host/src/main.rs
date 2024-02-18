use async_trait::async_trait;
use cap_std::fs::Dir;
use commander::base::{self, streams::HostOutputEventsStream, types::ValueEvent};
use wasmtime::{component::*, Config, Engine, Error, Store};
use wasmtime_wasi::preview2::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView};

bindgen!({
    world: "plugin",
    path: "../wit",
    async: true,
    with: {
        "wasi:io/error": wasmtime_wasi::preview2::bindings::io::error,
        "wasi:io/poll": wasmtime_wasi::preview2::bindings::io::poll,
        "wasi:io/streams": wasmtime_wasi::preview2::bindings::io::streams,
        "wasi:clocks/wall-clock": wasmtime_wasi::preview2::bindings::clocks::wall_clock,
        "wasi:filesystem/types": wasmtime_wasi::preview2::bindings::filesystem::types,
        "wasi:filesystem/preopens": wasmtime_wasi::preview2::bindings::filesystem::preopens,
    },
});

struct WasmStorage {
    table: ResourceTable,
    ctx: WasiCtx,
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
    fn new() -> Self {
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
        }
    }
}

impl base::types::Host for WasmStorage {}
impl base::streams::Host for WasmStorage {}

#[async_trait]
impl HostOutputEventsStream for WasmStorage {
    async fn send(
        &mut self,
        stream: wasmtime::component::Resource<OutputEventsStream>,
        event: ValueEvent,
    ) -> wasmtime::Result<(), Error> {
        println!("Sending event: {:?}", event);
        Ok(())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<OutputEventsStream>,
    ) -> wasmtime::Result<(), Error> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // Define the WASI functions globally on the `Config`.
    let engine = Engine::new(
        Config::default()
            .async_support(true)
            .wasm_component_model(true),
    )
    .unwrap();
    let mut linker: Linker<WasmStorage> = Linker::new(&engine);
    wasmtime_wasi::preview2::command::add_to_linker(&mut linker).unwrap();
    Plugin::add_to_linker(&mut linker, |w| w).unwrap();
    println!("Loading module");
    let ls_component = Component::from_file(&engine, "/Users/keatonbrandt/Documents/Development/Rust/commander/ls/target/wasm32-wasi/debug/ls.wasm").unwrap();

    println!("Instantiating module");
    let mut store = Store::new(&engine, WasmStorage::new());
    let (plugin, instance) = Plugin::instantiate_async(&mut store, &ls_component, &linker)
        .await
        .unwrap();

    println!("Loading Schema");
    println!(
        "Schema: {:?}",
        plugin.call_get_schema(&mut store).await.unwrap()
    );

    print!("Running program");
    return plugin
        .call_run(
            &mut store,
            vec![Value::PrimitiveValue(
                base::types::PrimitiveValue::PathValue(vec![
                    "Users".to_string(),
                    "keatonbrandt".to_string(),
                    "Documents".to_string(),
                ]),
            )]
            .as_slice(),
            Resource::new_own(1)
        )
        .await
        .unwrap()
        .map(|success| ());
}
