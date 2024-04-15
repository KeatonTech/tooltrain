use anyhow::{anyhow, Error};
use async_trait::async_trait;
use commander_data::{CommanderCoder, CommanderDataType, CommanderValue};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use wasmtime::component::Resource;

use crate::bindings::streaming::{ListInput, TreeInput};
use crate::bindings::streaming_inputs::{
    HostListChangeStream, HostListInput, HostTreeChangeStream, HostTreeInput,
    HostValueChangeStream, HostValueInput, ListChange, ListChangeStream, TreeChange,
    TreeChangeStream, TreeNode, ValueChangeStream, ValueInput,
};
use crate::datastream;
use crate::streaming::storage::DataStreamResourceChange;
use crate::streaming::WasmStorage;

#[async_trait]
impl HostValueInput for WasmStorage {
    async fn get(&mut self, resource: Resource<ValueInput>) -> Result<Option<Vec<u8>>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        let result = {
            let stream = data_stream_resource.stream.read();
            stream
                .try_get_value()?
                .snapshot()
                .map(|commander_value| data_type.encode((*commander_value).clone()))
                .transpose()
        };
        return result;
    }

    async fn get_change_stream(
        &mut self,
        resource: Resource<ValueInput>,
    ) -> Result<Resource<ValueChangeStream>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = data_stream_resource.metadata.data_type.clone();
        let resource_rep = resource.rep();

        let data_stream_change_stream = BroadcastStream::new(self.inputs.changes())
            .map_while(|result| result.ok())
            .filter(|change| change.is_data_stream_changed())
            .filter(move |change| {
                let DataStreamResourceChange::DataStreamChanged(changed_resource_id) = change
                else {
                    return false;
                };
                *changed_resource_id == resource_rep
            });

        let value_stream = BroadcastStream::new(
            data_stream_resource
                .stream
                .read()
                .try_get_value()?
                .subscribe(),
        )
        .filter_map(Result::ok)
        .filter_map(move |change| match change {
            datastream::ValueChange::Set(value) => Some(data_type.encode((*value).clone()).ok()),
            datastream::ValueChange::Destroy => None,
        });

        Ok(Resource::new_own(
            self.input_streams.value_streams.add_stream(
                resource_rep,
                value_stream,
                data_stream_change_stream,
            ),
        ))
    }

    async fn destroy(&mut self, resource: Resource<ValueInput>) -> Result<(), Error> {
        HostValueInput::drop(self, resource)
    }

    fn drop(&mut self, resource: Resource<ValueInput>) -> Result<(), Error> {
        if self.inputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent input"))
        }
    }
}

#[async_trait]
impl HostListInput for WasmStorage {
    async fn get(&mut self, resource: Resource<ListInput>) -> Result<Vec<u8>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        let CommanderDataType::List(list_data_type) = data_type else {
            return Err(anyhow!(
                "Expected a list<> data type, got {}",
                data_type.type_string()
            ));
        };
        let value_list: Vec<CommanderValue> = data_stream_resource
            .stream
            .read()
            .try_get_list()?
            .snapshot()
            .into_iter()
            .map(|v| (*v).clone())
            .collect();
        list_data_type.encode(value_list)
    }

    async fn request_more(
        &mut self,
        resource: Resource<ListInput>,
        limit: u32,
    ) -> Result<(), Error> {
        self.inputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_list_mut()?
            .request_page(limit)?;
        Ok(())
    }

    async fn get_change_stream(
        &mut self,
        resource: Resource<ListInput>,
    ) -> Result<Resource<ListChangeStream>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = data_stream_resource.metadata.data_type.clone();
        let resource_rep = resource.rep();

        let data_stream_change_stream = BroadcastStream::new(self.inputs.changes())
            .map_while(|result| result.ok())
            .filter(|change| change.is_data_stream_changed())
            .filter(move |change| {
                let DataStreamResourceChange::DataStreamChanged(changed_resource_id) = change
                else {
                    return false;
                };
                *changed_resource_id == resource_rep
            });

        let list_change_stream = BroadcastStream::new(
            data_stream_resource
                .stream
                .read()
                .try_get_list()?
                .subscribe(),
        )
        .filter_map(Result::ok)
        .map(
            move |data_stream_list_change| match data_stream_list_change {
                datastream::ListChange::Add(v) => {
                    ListChange::Append(data_type.encode((*v).clone()).unwrap())
                }
                datastream::ListChange::Pop(_) => ListChange::Pop,
                datastream::ListChange::HasMorePages(_) => todo!(),
                datastream::ListChange::Clear => ListChange::Replace(vec![]),
                datastream::ListChange::Destroy => todo!(),
            },
        );

        Ok(Resource::new_own(
            self.input_streams.list_streams.add_stream(
                resource.rep(),
                list_change_stream,
                data_stream_change_stream,
            ),
        ))
    }

    async fn destroy(&mut self, resource: Resource<ListInput>) -> Result<(), Error> {
        HostListInput::drop(self, resource)
    }

    fn drop(&mut self, resource: Resource<ListInput>) -> Result<(), Error> {
        if self.inputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent input"))
        }
    }
}

#[async_trait]
impl HostTreeInput for WasmStorage {
    async fn get(&mut self, resource: Resource<TreeInput>) -> Result<Vec<TreeNode>, Error> {
        Ok(self
            .inputs
            .get(resource.rep())?
            .stream
            .read()
            .try_get_tree()?
            .snapshot()
            .into_iter()
            .map(|tsn| (*tsn.value).clone())
            .collect())
    }

    async fn request_children(
        &mut self,
        resource: Resource<TreeInput>,
        of_parent: String,
    ) -> Result<(), Error> {
        self.inputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_tree_mut()?
            .request_children(of_parent)?;
        Ok(())
    }

    async fn get_change_stream(
        &mut self,
        resource: Resource<TreeInput>,
    ) -> Result<Resource<TreeChangeStream>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let resource_rep = resource.rep();

        let data_stream_change_stream = BroadcastStream::new(self.inputs.changes())
            .map_while(|result| result.ok())
            .filter(|change| change.is_data_stream_changed())
            .filter(move |change| {
                let DataStreamResourceChange::DataStreamChanged(changed_resource_id) = change
                else {
                    return false;
                };
                *changed_resource_id == resource_rep
            });

        let tree_change_stream = BroadcastStream::new(
            data_stream_resource
                .stream
                .read()
                .try_get_tree()?
                .subscribe(),
        )
        .filter_map(Result::ok)
        .map(
            move |data_stream_tree_change| match data_stream_tree_change {
                datastream::TreeChange::Add {
                    parent: _,
                    children,
                } => TreeChange::Append(children.iter().map(|a| (**a).clone()).collect()),
                datastream::TreeChange::Remove(node) => TreeChange::Remove(vec![node.id.clone()]),
                datastream::TreeChange::Clear => TreeChange::Replace(vec![]),
                datastream::TreeChange::Destroy => todo!(),
            },
        );

        Ok(Resource::new_own(
            self.input_streams.tree_streams.add_stream(
                resource.rep(),
                tree_change_stream,
                data_stream_change_stream,
            ),
        ))
    }

    async fn destroy(&mut self, resource: Resource<TreeInput>) -> Result<(), Error> {
        HostTreeInput::drop(self, resource)
    }

    fn drop(&mut self, resource: Resource<TreeInput>) -> Result<(), Error> {
        if self.inputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent input"))
        }
    }
}

#[async_trait]
impl HostValueChangeStream for WasmStorage {
    async fn poll_change(
        &mut self,
        resource: Resource<ValueChangeStream>,
    ) -> Result<Option<Option<Vec<u8>>>, Error> {
        self.input_streams
            .value_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Value change stream not found"))?
            .poll_change()
    }

    async fn poll_change_blocking(
        &mut self,
        resource: Resource<ValueChangeStream>,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.input_streams
            .value_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Value change stream not found"))?
            .poll_change_blocking(self.inputs.clone())
            .await
    }

    fn drop(&mut self, resource: Resource<ValueChangeStream>) -> Result<(), Error> {
        if self.input_streams.value_streams.remove(resource.rep()) {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent input"))
        }
    }
}

#[async_trait]
impl HostListChangeStream for WasmStorage {
    async fn poll_change(
        &mut self,
        resource: Resource<ListChangeStream>,
    ) -> Result<Option<ListChange>, Error> {
        self.input_streams
            .list_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("List change stream not found"))?
            .poll_change()
    }

    async fn poll_change_blocking(
        &mut self,
        resource: Resource<ListChangeStream>,
    ) -> Result<ListChange, Error> {
        self.input_streams
            .list_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("List change stream not found"))?
            .poll_change_blocking(self.inputs.clone())
            .await
    }

    fn drop(&mut self, resource: Resource<ListChangeStream>) -> Result<(), Error> {
        if self.input_streams.list_streams.remove(resource.rep()) {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent input"))
        }
    }
}

#[async_trait]
impl HostTreeChangeStream for WasmStorage {
    async fn poll_change(
        &mut self,
        resource: Resource<TreeChangeStream>,
    ) -> Result<Option<TreeChange>, Error> {
        self.input_streams
            .tree_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Tree change stream not found"))?
            .poll_change()
    }

    async fn poll_change_blocking(
        &mut self,
        resource: Resource<TreeChangeStream>,
    ) -> Result<TreeChange, Error> {
        self.input_streams
            .tree_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Tree change stream not found"))?
            .poll_change_blocking(self.inputs.clone())
            .await
    }

    fn drop(&mut self, resource: Resource<TreeChangeStream>) -> Result<(), Error> {
        if self.input_streams.tree_streams.remove(resource.rep()) {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent input"))
        }
    }
}
