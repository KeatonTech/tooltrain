use std::sync::Arc;

use crate::{
    bindings::streaming::{
        ListInput, ListOutput, StreamingPluginImports, TreeInput, TreeOutput, ValueInput,
        ValueOutput,
    },
    datastream::{DataStream, ListStream, TreeStream, ValueStream},
    streaming::storage::WasmStorage,
};

use anyhow::Error;
use async_trait::async_trait;

use commander_data::{parse, CommanderCoder};
use parking_lot::RwLock;
use wasmtime::component::*;

#[async_trait]
impl StreamingPluginImports for WasmStorage {
    async fn add_value_output(
        &mut self,
        name: String,
        description: String,
        data_type: String,
        initial_value: Option<Vec<u8>>,
    ) -> Result<Resource<ValueOutput>, Error> {
        let commander_data_type = parse(&data_type)?;
        let decoded_initial_value = if let Some(bytes) = initial_value {
            Some(commander_data_type.decode(&bytes)?)
        } else {
            None
        };

        Ok(Resource::new_own(self.outputs.add(
            name,
            description,
            commander_data_type,
            Arc::new(RwLock::new(DataStream::Value(ValueStream::new(
                decoded_initial_value,
            )))),
        )?))
    }

    async fn add_list_output(
        &mut self,
        name: String,
        description: String,
        data_type: String,
    ) -> Result<Resource<ListOutput>, Error> {
        Ok(Resource::new_own(self.outputs.add(
            name,
            description,
            parse(&data_type)?,
            Arc::new(RwLock::new(DataStream::List(ListStream::new()))),
        )?))
    }

    async fn add_tree_output(
        &mut self,
        name: String,
        description: String,
        data_type: String,
    ) -> Result<Resource<TreeOutput>, Error> {
        Ok(Resource::new_own(self.outputs.add(
            name,
            description,
            parse(&data_type)?,
            Arc::new(RwLock::new(DataStream::Tree(TreeStream::new()))),
        )?))
    }

    async fn add_value_input(
        &mut self,
        name: String,
        description: String,
        data_type: String,
        initial_value: Option<Vec<u8>>,
    ) -> Result<Resource<ValueInput>, Error> {
        let commander_data_type = parse(&data_type)?;
        let decoded_initial_value = if let Some(bytes) = initial_value {
            Some(commander_data_type.decode(&bytes)?)
        } else {
            None
        };

        Ok(Resource::new_own(self.inputs.add(
            name,
            description,
            commander_data_type,
            Arc::new(RwLock::new(DataStream::Value(ValueStream::new(
                decoded_initial_value,
            )))),
        )?))
    }

    async fn add_list_input(
        &mut self,
        name: String,
        description: String,
        data_type: String,
    ) -> Result<Resource<ListInput>, Error> {
        Ok(Resource::new_own(self.inputs.add(
            name,
            description,
            parse(&data_type)?,
            Arc::new(RwLock::new(DataStream::List(ListStream::new()))),
        )?))
    }

    async fn add_tree_input(
        &mut self,
        name: String,
        description: String,
        data_type: String,
    ) -> Result<Resource<TreeInput>, Error> {
        Ok(Resource::new_own(self.inputs.add(
            name,
            description,
            parse(&data_type)?,
            Arc::new(RwLock::new(DataStream::Tree(TreeStream::new()))),
        )?))
    }
}

impl crate::bindings::streaming::commander::base::inputs::Host for WasmStorage {}
impl crate::bindings::streaming::commander::base::streaming_inputs::Host for WasmStorage {}
impl crate::bindings::streaming::commander::base::streaming_outputs::Host for WasmStorage {}
