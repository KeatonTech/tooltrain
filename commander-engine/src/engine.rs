use std::{future::Future, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Error};

use tokio::sync::watch;

use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

use crate::{
    bindings::{Plugin, Schema, Value},
    outputs::Outputs,
    storage::WasmStorage,
};

struct CommanderEngineInternal {
    wasm_engine: Engine,
    linker: Linker<WasmStorage>,
}

impl Default for CommanderEngineInternal {
    fn default() -> Self {
        let engine = Engine::new(
            Config::default()
                .async_support(true)
                .wasm_component_model(true),
        )
        .unwrap();

        let mut linker: Linker<WasmStorage> = Linker::new(&engine);
        wasmtime_wasi::preview2::command::add_to_linker(&mut linker).unwrap();
        wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |c| c).unwrap();
        wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |c| c)
            .unwrap();
        Plugin::add_to_linker(&mut linker, |w| w).unwrap();

        CommanderEngineInternal {
            wasm_engine: engine,
            linker,
        }
    }
}

pub struct CommanderEngine(Arc<CommanderEngineInternal>);

impl Default for CommanderEngine {
    fn default() -> Self {
        Self(Arc::new(Default::default()))
    }
}

pub enum ProgramSource {
    FilePath(PathBuf),
}

impl ProgramSource {
    fn open(&self, engine: &CommanderEngineInternal) -> Result<Component, Error> {
        match self {
            ProgramSource::FilePath(path) => Component::from_file(&engine.wasm_engine, path),
        }
    }
}

impl CommanderEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open_program(&self, program: ProgramSource) -> Result<CommanderProgram, Error> {
        let component = program.open(&self.0)?;
        Ok(CommanderProgram {
            engine: self.0.clone(),
            component,
        })
    }
}

pub struct CommanderProgram {
    engine: Arc<CommanderEngineInternal>,
    component: Component,
}

impl CommanderProgram {
    pub async fn get_schema(&mut self) -> Result<Schema, Error> {
        let (mut store, program) = self.load_instance().await?;
        program.call_get_schema(&mut store).await
    }

    pub async fn run(&mut self, arguments: Vec<Value>) -> Result<CommanderProgramRun, Error> {
        let (store, plugin) = self.load_instance().await?;
        let outputs = store.data().get_outputs();
        let run_result = CommanderProgram::run_wrapper(store, plugin, arguments);
        Ok(CommanderProgramRun::new(outputs, run_result))
    }

    async fn load_instance(&mut self) -> Result<(Store<WasmStorage>, Plugin), Error> {
        let mut store = Store::new(&self.engine.wasm_engine, WasmStorage::new());
        let (plugin, _) =
            Plugin::instantiate_async(&mut store, &self.component, &self.engine.linker).await?;
        Ok((store, plugin))
    }

    async fn run_wrapper(
        mut store: Store<WasmStorage>,
        plugin: Plugin,
        arguments: Vec<Value>,
    ) -> Result<Result<String, String>, Error> {
        plugin.call_run(&mut store, arguments.as_slice()).await
    }
}

pub struct CommanderProgramRun {
    outputs: Outputs,
    result_reader: watch::Receiver<Option<Arc<Result<String, Error>>>>,
}

impl CommanderProgramRun {
    fn new(
        outputs: Outputs,
        run_future: impl Future<Output = Result<Result<String, String>, Error>> + Send + 'static,
    ) -> Self {
        let (result_writer, result_reader) = watch::channel(None);
        tokio::spawn(async move {
            let result = run_future
                .await
                .and_then(|r| r.map_err(|e| anyhow!("Program ended with an error: {}", e)));
            result_writer.send(Some(Arc::new(result))).unwrap();
        });
        Self {
            outputs,
            result_reader,
        }
    }

    pub async fn get_result(&mut self) -> Arc<Result<String, Error>> {
        if self.result_reader.borrow().is_none() {
            self.result_reader.changed().await.unwrap();
        }
        self.result_reader.borrow().as_ref().unwrap().clone()
    }

    pub fn outputs(&self) -> Outputs {
        self.outputs.clone()
    }
}
