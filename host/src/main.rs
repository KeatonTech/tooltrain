use std::{collections::BTreeMap, process::Output};

use __internal::anyhow::anyhow;
use async_trait::async_trait;
use cap_std::fs::Dir;
use commander::base::{self, streams::HostOutputHandle, types::ValueEvent};
use tokio::sync::broadcast::{channel, Sender};
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

struct OutputSpec {
    name: String,
    description: String,
    data_type: DataType,
    value: Option<Value>,
    updates: Sender<ValueEvent>,
}

struct WasmStorage {
    table: ResourceTable,
    ctx: WasiCtx,
    outputs: BTreeMap<u32, OutputSpec>,
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
            outputs: BTreeMap::new(),
        }
    }
}

impl base::types::Host for WasmStorage {}
impl base::streams::Host for WasmStorage {}

impl base::streams::HostRunHandle for WasmStorage {
    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<base::streams::RunHandle>,
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
        let output = self.outputs.get_mut(&handle.rep()).unwrap();
        match event.clone() {
            ValueEvent::Add(value) => match &mut output.value {
                Some(Value::TableValue(existing_table)) => {
                    if let Value::TableValue(added) = value {
                        existing_table.extend(added.into_iter());
                    } else {
                        return Err(anyhow!("Add must contain a TableValue"));
                    }
                }
                None => {
                    output.value = Some(value);
                }
                _ => {
                    return Err(anyhow!("Invalid value type"));
                }
            },
            ValueEvent::Set(value) => {
                output.value = Some(value);
            }
            ValueEvent::Clear => {
                output.value = None;
            }
        }
        println!("Updated Value: {:?}", output.value);
        let _ = output.updates.send(event);
        Ok(())
    }

    async fn remove(&mut self, handle: Resource<OutputHandle>) -> wasmtime::Result<(), Error> {
        drop(handle);
        Ok(())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<OutputHandle>,
    ) -> wasmtime::Result<(), Error> {
        Ok(())
    }
}

#[async_trait]
impl PluginImports for WasmStorage {
    async fn add_output(
        &mut self,
        run_handle: Resource<RunHandle>,
        name: String,
        description: String,
        data_type: DataType,
        value: Option<Value>,
    ) -> Result<Resource<OutputHandle>, Error> {
        let next_index = self
            .outputs
            .last_key_value()
            .map(|(k, _)| k + 1)
            .unwrap_or(0);
        let (tx, _) = channel(1);
        println!("Adding output: {name}");
        self.outputs.insert(
            next_index,
            OutputSpec {
                name,
                description,
                data_type,
                value,
                updates: tx,
            },
        );
        Ok(Resource::new_own(next_index))
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
    let ls_component = Component::from_file(
        &engine,
        "/Users/keatonbrandt/Documents/Development/Rust/commander/target/wasm32-wasi/debug/ls.wasm",
    )
    .unwrap();

    println!("Instantiating module");
    let mut store = Store::new(&engine, WasmStorage::new());
    let (plugin, _) = Plugin::instantiate_async(&mut store, &ls_component, &linker)
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
            Resource::new_own(0),
        )
        .await
        .unwrap()
        .map(|success| {
            println!("Final output: {}", success);
            ()
        });
}
