use anyhow::{anyhow, Error};
use async_trait::async_trait;
use commander_data::{CommanderCoder, CommanderDataType, CommanderValue};
use tokio::sync::broadcast::error::TryRecvError;
use wasmtime::component::Resource;

use crate::bindings::streaming::{ListInput, TreeInput};
use crate::bindings::streaming_inputs::{
    self, HostListInput, HostTreeInput, HostValueInput, TreeNode, ValueInput,
};
use crate::datastream::{ListChange, TreeChange, ValueChange};
use crate::streaming::WasmStorage;

#[async_trait]
impl HostValueInput for WasmStorage {
    async fn get(&mut self, resource: Resource<ValueInput>) -> Result<Option<Vec<u8>>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        data_stream_resource
            .stream
            .try_get_value()?
            .snapshot()
            .map(|commander_value| data_type.encode((*commander_value).clone()))
            .transpose()
    }

    async fn poll_change(
        &mut self,
        resource: Resource<ValueInput>,
    ) -> Result<Option<Option<Vec<u8>>>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        let value_change = data_stream_resource
            .stream
            .try_get_value()?
            .subscribe()
            .try_recv();
        match value_change {
            Ok(ValueChange::Set(commander_value)) => {
                Ok(Some(Some(data_type.encode((*commander_value).clone())?)))
            }
            Ok(ValueChange::Destroy) => Err(anyhow!("Input was destroyed")),
            Err(TryRecvError::Empty) => Ok(None),
            Err(e) => Err(anyhow!("RecvError when reading from ValueStream. {:?}", e)),
        }
    }

    async fn poll_change_blocking(
        &mut self,
        resource: Resource<ValueInput>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        let value_change = data_stream_resource
            .stream
            .try_get_value()?
            .subscribe()
            .recv()
            .await;
        match value_change {
            Ok(ValueChange::Set(commander_value)) => {
                Ok(Some(data_type.encode((*commander_value).clone())?))
            }
            Ok(ValueChange::Destroy) => Err(anyhow!("Input was destroyed")),
            Err(e) => Err(anyhow!("RecvError when reading from ValueStream. {:?}", e)),
        }
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
            .get_mut(resource.rep())?
            .stream
            .try_get_list_mut()?
            .request_page(limit)?;
        Ok(())
    }

    async fn poll_change(
        &mut self,
        resource: Resource<ListInput>,
    ) -> Result<Option<streaming_inputs::ListChange>, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        let CommanderDataType::List(list_data_type) = data_type else {
            return Err(anyhow!(
                "Expected a list<> data type, got {}",
                data_type.type_string()
            ));
        };
        let list_change = data_stream_resource
            .stream
            .try_get_list()?
            .subscribe()
            .try_recv();
        match list_change {
            Ok(ListChange::Add(commander_value)) => Ok(Some(streaming_inputs::ListChange::Append(
                data_type.encode((*commander_value).clone())?,
            ))),
            Ok(ListChange::Pop(_)) => Ok(Some(streaming_inputs::ListChange::Pop)),
            Ok(ListChange::Clear) => Ok(Some(streaming_inputs::ListChange::Replace(
                list_data_type.encode(vec![])?,
            ))),
            Ok(ListChange::HasMorePages(has_more_pages)) => Ok(Some(
                streaming_inputs::ListChange::HasMorePages(has_more_pages),
            )),
            Ok(ListChange::Destroy) => Err(anyhow!("Input was destroyed")),
            Err(TryRecvError::Empty) => Ok(None),
            Err(e) => Err(anyhow!("RecvError when reading from ValueStream. {:?}", e)),
        }
    }

    async fn poll_change_blocking(
        &mut self,
        resource: Resource<ListInput>,
    ) -> Result<streaming_inputs::ListChange, Error> {
        let data_stream_resource = self.inputs.get(resource.rep())?;
        let data_type = &data_stream_resource.metadata.data_type;
        let CommanderDataType::List(list_data_type) = data_type else {
            return Err(anyhow!(
                "Expected a list<> data type, got {}",
                data_type.type_string()
            ));
        };
        let list_change = data_stream_resource
            .stream
            .try_get_list()?
            .subscribe()
            .recv()
            .await;
        match list_change {
            Ok(ListChange::Add(commander_value)) => Ok(streaming_inputs::ListChange::Append(
                data_type.encode((*commander_value).clone())?,
            )),
            Ok(ListChange::Pop(_)) => Ok(streaming_inputs::ListChange::Pop),
            Ok(ListChange::Clear) => Ok(streaming_inputs::ListChange::Replace(
                list_data_type.encode(vec![])?,
            )),
            Ok(ListChange::HasMorePages(has_more_pages)) => {
                Ok(streaming_inputs::ListChange::HasMorePages(has_more_pages))
            }
            Ok(ListChange::Destroy) => Err(anyhow!("Input was destroyed")),
            Err(e) => Err(anyhow!("RecvError when reading from ValueStream. {:?}", e)),
        }
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
            .get_mut(resource.rep())?
            .stream
            .try_get_tree_mut()?
            .request_children(of_parent)?;
        Ok(())
    }

    async fn poll_change(
        &mut self,
        resource: Resource<TreeInput>,
    ) -> Result<Option<streaming_inputs::TreeChange>, Error> {
        let data_stream_resource = self
            .inputs
            .get(resource.rep())?;
        let tree_change = data_stream_resource
            .stream
            .try_get_tree()?
            .subscribe()
            .try_recv();
        match tree_change {
            Ok(TreeChange::Add { parent, children }) => {
                Ok(Some(streaming_inputs::TreeChange::Append(
                    children.into_iter().map(|c| (*c).clone()).collect(),
                )))
            }
            Ok(TreeChange::Remove(removed)) => {
                Ok(Some(streaming_inputs::TreeChange::Remove(vec![removed
                    .id
                    .clone()])))
            }
            Ok(TreeChange::Clear) => Ok(Some(streaming_inputs::TreeChange::Replace(vec![]))),
            Ok(TreeChange::Destroy) => Err(anyhow!("Input was destroyed")),
            Err(TryRecvError::Empty) => Ok(None),
            Err(e) => Err(anyhow!("RecvError when reading from ValueStream. {:?}", e)),
        }
    }

    async fn poll_change_blocking(
        &mut self,
        resource: Resource<TreeInput>,
    ) -> Result<streaming_inputs::TreeChange, Error> {
        let data_stream_resource = self
            .inputs
            .get(resource.rep())?;
        let tree_change = data_stream_resource
            .stream
            .try_get_tree()?
            .subscribe()
            .recv()
            .await;
        match tree_change {
            Ok(TreeChange::Add { parent, children }) => Ok(streaming_inputs::TreeChange::Append(
                children.into_iter().map(|c| (*c).clone()).collect(),
            )),
            Ok(TreeChange::Remove(removed)) => {
                Ok(streaming_inputs::TreeChange::Remove(vec![removed
                    .id
                    .clone()]))
            }
            Ok(TreeChange::Clear) => Ok(streaming_inputs::TreeChange::Replace(vec![])),
            Ok(TreeChange::Destroy) => Err(anyhow!("Input was destroyed")),
            Err(e) => Err(anyhow!("RecvError when reading from ValueStream. {:?}", e)),
        }
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
