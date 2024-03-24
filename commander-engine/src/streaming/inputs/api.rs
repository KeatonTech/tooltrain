use std::marker::PhantomData;

use commander_data::{CommanderCoder, CommanderDataType, CommanderValue};
use wasmtime::component::Resource;

use crate::{
    bindings,
    datastream::{DataStream, ValueStream},
    streaming::{
        storage::{DataStreamMetadata, ResourceId},
        DataStreamStorage,
    },
};
use anyhow::Error;

pub(crate) trait InputHandle {
    fn as_input_binding(&self) -> bindings::streaming_inputs::Input;
}

#[derive(Clone, Debug)]
pub struct ValueInputHandle<ValueType: CommanderCoder> {
    pub metadata: DataStreamMetadata,
    value_type: std::marker::PhantomData<ValueType>,
}

impl<ValueType: CommanderCoder> InputHandle for ValueInputHandle<ValueType> {
    fn as_input_binding(&self) -> bindings::streaming_inputs::Input {
        let value_resource: Resource<bindings::streaming_inputs::ValueInput> =
            Resource::new_own(self.metadata.id);
        bindings::streaming_inputs::Input::ValueInput(value_resource)
    }
}

impl<ValueType: CommanderCoder> ValueInputHandle<ValueType> {
    pub fn load<'a>(&self, from_storage: Inputs<'a>) -> ValueInputRef<'a, ValueType> {
        ValueInputRef {
            storage: from_storage.0,
            id: self.metadata.id,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct ValueInputRef<'a, ValueType: CommanderCoder> {
    storage: &'a DataStreamStorage,
    id: ResourceId,
    _phantom: PhantomData<ValueType>,
}

impl<'a, ValueType: CommanderCoder> ValueInputRef<'a, ValueType>
where
    ValueType::Value: Into<CommanderValue>,
{
    pub fn set(&self, value: ValueType::Value) -> Result<(), Error> {
        self.storage
            .get_mut(self.id)?
            .stream
            .try_get_value_mut()?
            .set(value.into())
    }
}

pub struct Inputs<'a>(pub(crate) &'a DataStreamStorage);

impl<'a> Inputs<'a> {
    pub fn new_value_input<ValueType>(
        &self,
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
        let resource_id = self.0.add(
            name,
            description,
            data_type.into(),
            DataStream::Value(ValueStream::new(initial_value.map(|v| v.into()))),
        )?;
        Ok(ValueInputHandle {
            metadata: self.0.get(resource_id).unwrap().metadata.clone(),
            value_type: PhantomData,
        })
    }
}
