use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use tokio_stream::{once, wrappers::BroadcastStream, Stream, StreamExt};
use tooltrain_data::{
    TooltrainCoder, TooltrainDataType, TooltrainListDataType, TooltrainTypedListDataType,
    TooltrainValue,
};
use wasmtime::component::Resource;

use crate::{
    bindings,
    datastream::{DataStream, DataStreamSnapshot, ListStream, ValueStream},
    streaming::{
        storage::{DataStreamMetadata, DataStreamResourceChange, DataStreamType, ResourceId},
        DataStreamStorage, ListOutputRef, OutputRef, ValueOutputRef,
    },
};
use anyhow::Error;

#[derive(Clone, Debug)]
pub struct ValueInputHandle<ValueType: TooltrainCoder> {
    pub metadata: DataStreamMetadata,
    value_type: std::marker::PhantomData<ValueType>,
}

impl<ValueType: TooltrainCoder> ValueInputHandle<ValueType> {
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

    pub fn downcast<T: TooltrainCoder>(&self) -> ValueInputHandle<T>
    where
        T: Into<ValueType>,
    {
        ValueInputHandle {
            metadata: self.metadata.clone(),
            value_type: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct ValueInputRef<'a, ValueType: TooltrainCoder> {
    storage: &'a DataStreamStorage,
    id: ResourceId,
    _phantom: PhantomData<ValueType>,
}

impl<ValueType: TooltrainCoder> ValueInputRef<'_, ValueType>
where
    ValueType::Value: Into<TooltrainValue>,
{
    pub fn set(&self, value: ValueType::Value) -> Result<(), Error> {
        self.storage
            .get(self.id)?
            .stream
            .write()
            .try_get_value_mut()?
            .set(value.into())
    }

    pub fn bind(&self, from: ValueOutputRef<'_>) -> Result<(), Error> {
        self.storage
            .change_data_stream(self.id, from.inner_data_stream()?)
    }
}

#[derive(Clone, Debug)]
pub struct ListInputHandle<ValueType: TooltrainCoder> {
    pub metadata: DataStreamMetadata,
    value_type: std::marker::PhantomData<ValueType>,
}

impl<ValueType: TooltrainCoder> ListInputHandle<ValueType> {
    pub(crate) fn as_input_binding(&self) -> bindings::streaming_inputs::Input {
        let value_resource: Resource<bindings::streaming_inputs::ListInput> =
            Resource::new_own(self.metadata.id);
        bindings::streaming_inputs::Input::ListInput(value_resource)
    }

    pub fn load<'a>(&self, from_storage: Inputs<'a>) -> ListInputRef<'a, ValueType> {
        ListInputRef {
            storage: from_storage.0,
            id: self.metadata.id,
            _phantom: PhantomData,
        }
    }

    pub fn downcast<T: TooltrainCoder>(&self) -> ListInputHandle<T>
    where
        T: Into<ValueType>,
    {
        ListInputHandle {
            metadata: self.metadata.clone(),
            value_type: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct ListInputRef<'a, ValueType: TooltrainCoder> {
    storage: &'a DataStreamStorage,
    id: ResourceId,
    _phantom: PhantomData<ValueType>,
}

impl<ValueType: TooltrainCoder> ListInputRef<'_, ValueType>
where
    ValueType::Value: Into<TooltrainValue>,
{
    pub fn add(&self, value: ValueType::Value) -> Result<(), Error> {
        self.storage
            .get(self.id)?
            .stream
            .write()
            .try_get_list_mut()?
            .add(value.into())
    }

    pub fn bind(&self, from: ListOutputRef<'_>) -> Result<(), Error> {
        self.storage
            .change_data_stream(self.id, from.inner_data_stream()?)
    }
}

#[derive(Clone, Debug)]
pub enum InputHandle {
    Value(ValueInputHandle<TooltrainDataType>),
    List(ListInputHandle<TooltrainDataType>),
}

impl InputHandle {
    fn from_metadata(metadata: DataStreamMetadata) -> Self {
        match metadata.data_stream_type {
            DataStreamType::Value => InputHandle::Value(ValueInputHandle::<TooltrainDataType> {
                metadata,
                value_type: PhantomData,
            }),
            DataStreamType::List => InputHandle::List(ListInputHandle::<TooltrainDataType> {
                metadata,
                value_type: PhantomData,
            }),
            _ => unimplemented!(),
        }
    }

    fn metadata(&self) -> &DataStreamMetadata {
        match self {
            InputHandle::Value(handle) => &handle.metadata,
            InputHandle::List(handle) => &handle.metadata,
        }
    }

    pub(crate) fn as_input_binding(&self) -> bindings::streaming_inputs::Input {
        match self {
            InputHandle::Value(handle) => handle.as_input_binding(),
            InputHandle::List(handle) => handle.as_input_binding(),
        }
    }
}

#[derive(Debug)]
pub enum InputChange {
    Added(InputHandle),
    Removed(ResourceId),
}

pub struct Inputs<'a>(pub(crate) &'a DataStreamStorage);

impl Inputs<'_> {
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

    pub fn get_handle(&self, input_name: &str) -> Option<InputHandle> {
        self.handles()
            .into_iter()
            .find(|handle| handle.metadata().name == input_name)
    }

    pub fn new_value_input<ValueType>(
        &self,
        name: String,
        description: String,
        data_type: ValueType,
        initial_value: Option<ValueType::Value>,
    ) -> Result<ValueInputHandle<ValueType>, Error>
    where
        ValueType: TooltrainCoder,
        ValueType: Into<TooltrainDataType>,
        ValueType::Value: Into<TooltrainValue>,
    {
        let resource_id = self.0.add(
            name,
            description,
            data_type.into(),
            Arc::new(RwLock::new(DataStream::Value(ValueStream::new(
                initial_value.map(|v| v.into()),
            )))),
        )?;
        Ok(ValueInputHandle {
            metadata: self.0.get(resource_id).unwrap().metadata.clone(),
            value_type: PhantomData,
        })
    }

    pub fn new_list_input<V: TooltrainCoder + 'static>(
        &self,
        name: String,
        description: String,
        data_type: TooltrainTypedListDataType<V>,
    ) -> Result<ListInputHandle<V>, Error>
    where
        TooltrainTypedListDataType<V>: Into<TooltrainListDataType>,
    {
        let resource_id = self.0.add(
            name,
            description,
            TooltrainDataType::List(data_type.into()),
            Arc::new(RwLock::new(DataStream::List(ListStream::new()))),
        )?;
        Ok(ListInputHandle {
            metadata: self.0.get(resource_id).unwrap().metadata.clone(),
            value_type: PhantomData,
        })
    }

    pub(crate) fn new_generic_list_input(
        &self,
        name: String,
        description: String,
        data_type: TooltrainListDataType,
    ) -> Result<ListInputHandle<TooltrainDataType>, Error> {
        let resource_id = self.0.add(
            name,
            description,
            TooltrainDataType::List(data_type),
            Arc::new(RwLock::new(DataStream::List(ListStream::new()))),
        )?;
        Ok(ListInputHandle {
            metadata: self.0.get(resource_id).unwrap().metadata.clone(),
            value_type: PhantomData,
        })
    }

    pub fn bind_input<ValueType, O: OutputRef>(
        &self,
        name: String,
        description: String,
        data_type: ValueType,
        from: O,
    ) -> Result<InputHandle, Error>
    where
        ValueType: TooltrainCoder,
        ValueType: Into<TooltrainDataType>,
        ValueType::Value: Into<TooltrainValue>,
    {
        let resource_id = self.0.add(
            name,
            description,
            data_type.into(),
            from.inner_data_stream()?.clone(),
        )?;
        Ok(InputHandle::from_metadata(
            self.0.get(resource_id).unwrap().metadata.clone(),
        ))
    }
}
