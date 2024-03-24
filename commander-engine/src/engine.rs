use std::{future::Future, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Error};

use commander_data::{CommanderCoder, CommanderDataType, CommanderValue};
use parking_lot::RwLock;
use tokio::sync::watch;

use wasmtime::{
    component::{Component, Linker},
    AsContext, Config, Engine, Store,
};

use crate::{
    bindings::{
        inputs,
        streaming::{Input, StreamingPlugin},
    },
    datastream::DataStream,
    streaming::{DataStreamStorage, InputHandle, Inputs, Outputs, ValueInputHandle, WasmStorage},
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
        wasmtime_wasi::command::add_to_linker(&mut linker).unwrap();
        wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |c| c).unwrap();
        wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |c| c)
            .unwrap();
        StreamingPlugin::add_to_linker(&mut linker, |w| w).unwrap();

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

    pub async fn open_program(
        &self,
        program: ProgramSource,
    ) -> Result<CommanderStreamingProgram, Error> {
        let component = program.open(&self.0)?;
        Ok(CommanderStreamingProgram {
            engine: self.0.clone(),
            component,
        })
    }
}

pub struct CommanderStreamingProgram {
    engine: Arc<CommanderEngineInternal>,
    component: Component,
}

impl CommanderStreamingProgram {
    pub async fn get_schema(&mut self) -> Result<inputs::Schema, Error> {
        let (mut store, program) = self.load_instance().await?;
        program.call_get_schema(&mut store).await
    }

    pub async fn run(&mut self) -> Result<StreamingRunBuilder, Error> {
        StreamingRunBuilder::new(self).await
    }

    async fn load_instance(&mut self) -> Result<(Store<WasmStorage>, StreamingPlugin), Error> {
        let mut store = Store::new(&self.engine.wasm_engine, WasmStorage::new());
        let (plugin, _) =
            StreamingPlugin::instantiate_async(&mut store, &self.component, &self.engine.linker)
                .await?;
        Ok((store, plugin))
    }
}

pub struct StreamingRunBuilder {
    instance: StreamingPlugin,
    store: Store<WasmStorage>,
    inputs: Vec<Input>,
}

impl StreamingRunBuilder {
    pub async fn new(program: &mut CommanderStreamingProgram) -> Result<Self, Error> {
        let (store, instance) = program.load_instance().await?;
        Ok(Self {
            instance,
            store,
            inputs: vec![],
        })
    }

    pub fn add_value_input<ValueType>(
        &mut self,
        name: String,
        description: String,
        data_type: ValueType,
        initial_value: Option<ValueType::Value>,
    ) -> Result<ValueInputHandle<ValueType>, Error>
    where
        ValueType: CommanderCoder,
        ValueType: Into<CommanderDataType>,
        ValueType::Value: Into<CommanderValue>,
    {
        let inputs = Inputs(&self.store.data().inputs);
        let input_handle = inputs.new_value_input(name, description, data_type, initial_value)?;
        self.inputs.push(input_handle.as_input_binding());
        Ok(input_handle)
    }

    pub fn with_static_input<ValueType>(
        mut self,
        name: String,
        description: String,
        data_type: ValueType,
        value: ValueType::Value,
    ) -> Result<Self, Error>
    where
        ValueType: CommanderCoder,
        ValueType: Into<CommanderDataType>,
        ValueType::Value: Into<CommanderValue>,
    {
        let _ = self.add_value_input(name, description, data_type, Some(value))?;
        Ok(self)
    }

    pub fn start(self) -> CommanderStreamingProgramRun {
        let Self {
            instance,
            store,
            inputs,
        } = self;
        let inputs_storage = store.data().inputs.clone();
        let outputs_storage = store.data().outputs.clone();
        let run_result = Self::run_wrapper(store, instance, inputs);
        CommanderStreamingProgramRun::new(inputs_storage, outputs_storage, run_result)
    }

    async fn run_wrapper(
        mut store: Store<WasmStorage>,
        plugin: StreamingPlugin,
        arguments: Vec<Input>,
    ) -> Result<Result<String, String>, Error> {
        plugin.call_run(&mut store, arguments.as_slice()).await
    }
}

#[derive(Debug, Clone)]
pub struct CommanderStreamingProgramRun {
    inputs: DataStreamStorage,
    outputs: DataStreamStorage,
    result_reader: watch::Receiver<Option<Arc<Result<String, Error>>>>,
}

impl CommanderStreamingProgramRun {
    fn new(
        inputs: DataStreamStorage,
        outputs: DataStreamStorage,
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
            inputs,
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

    pub fn outputs(&self) -> Outputs<'_> {
        Outputs(&self.outputs)
    }

    pub fn inputs(&self) -> Inputs<'_> {
        Inputs(&self.inputs)
    }
}
