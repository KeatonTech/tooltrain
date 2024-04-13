use std::{collections::BTreeMap, marker::PhantomData};

use commander_data::{CommanderCoder, CommanderDataType, CommanderValue};
use tokio_stream::{once, wrappers::BroadcastStream, Stream, StreamExt};
use wasmtime::component::Resource;

use crate::{
    bindings::{self, streaming_inputs::Input},
    datastream::{DataStream, DataStreamSnapshot, ValueStream},
    streaming::{
        storage::{DataStreamMetadata, DataStreamResourceChange, DataStreamType, ResourceId},
        DataStreamStorage, ValueOutputRef,
    },
};
use anyhow::Error;

#[derive(Clone, Debug)]
pub struct ValueInputHandle<ValueType: CommanderCoder> {
    pub metadata: DataStreamMetadata,
    value_type: std::marker::PhantomData<ValueType>,
}

impl<ValueType: CommanderCoder> ValueInputHandle<ValueType> {
    pub(crate) fn as_input_binding(&self) -> bindings::streaming_inputs::Input {
        let value_resource: Resource<bindings::streaming_inputs::ValueInput> =
            Resource::new_own(self.metadata.id);
        bindings::streaming_inputs::Input::ValueInput(value_resource)
    }

    pub fn load<'a>(&self, from_storage: Inputs<'a>) -> ValueInputRef<'a, ValueType> {
        ValueInputRef {
            storage: from_storage.0,
            id: self.metadata.id,
            _phantom: PhantomData,
        }
    }

    pub fn downcast<T: CommanderCoder> (&self) -> ValueInputHandle<T> where T: Into<ValueType> {
        ValueInputHandle {
            metadata: self.metadata.clone(),
            value_type: PhantomData
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
            .get(self.id)?
            .stream
            .write()
            .try_get_value_mut()?
            .set(value.into())
    }

    pub fn pipe(&self, from: ValueOutputRef<'_>) -> Result<(), Error> {
        self.storage
            .change_data_stream(self.id, from.inner_data_stream()?)
    }
}

#[derive(Clone, Debug)]
pub enum InputHandle {
    Value(ValueInputHandle<CommanderDataType>),
}

impl InputHandle {
    fn from_metadata(metadata: DataStreamMetadata) -> Self {
        match metadata.data_stream_type {
            DataStreamType::Value => InputHandle::Value(ValueInputHandle::<CommanderDataType> {
                metadata,
                value_type: PhantomData,
            }),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum InputChange {
    Added(InputHandle),
    Removed(ResourceId),
}

pub struct Inputs<'a>(pub(crate) &'a DataStreamStorage);

impl<'a> Inputs<'a> {
    pub fn updates(&self) -> impl Stream<Item = InputChange> + '_ {
        BroadcastStream::from(self.0.changes())
            .map_while(|result| result.ok())
            .filter_map(|internal_change| match internal_change {
                DataStreamResourceChange::Added(metadata) => {
                    Some(InputChange::Added(InputHandle::from_metadata(metadata)))
                }
                DataStreamResourceChange::Removed(id) => Some(InputChange::Removed(id)),
                DataStreamResourceChange::DataStreamChanged(_) => None,
            })
    }

    pub fn handles_stream(&self) -> impl Stream<Item = Vec<InputHandle>> + '_ {
        once(self.handles()).chain(self.updates().map(|_| self.handles()))
    }

    pub fn handles(&self) -> Vec<InputHandle> {
        self.0
            .state()
            .values()
            .map(|state| InputHandle::from_metadata(state.metadata.clone()))
            .collect()
    }

    pub fn values(&self) -> BTreeMap<ResourceId, DataStreamSnapshot> {
        self.0
            .state()
            .iter()
            .map(|(id, spec)| (*id, spec.stream.read().snapshot()))
            .collect()
    }

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
