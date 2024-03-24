use crate::{
    bindings::{
        inputs::TreeNode,
        streaming::{ListOutput, TreeOutput, ValueOutput},
        streaming_outputs::{
            HostListOutput, HostTreeOutput, HostValueOutput, ListOutputRequest, TreeOutputRequest,
        },
    },
    streaming::storage::WasmStorage,
};

use anyhow::{anyhow, Error};
use async_trait::async_trait;

use commander_data::{
    CommanderCoder, CommanderDataType
};
use wasmtime::component::*;

#[async_trait]
impl HostValueOutput for WasmStorage {
    async fn set(&mut self, resource: Resource<ValueOutput>, value: Vec<u8>) -> Result<(), Error> {
        let data_type = &self
            .outputs
            .get(resource.rep())?
            .metadata
            .data_type;
        self.outputs
            .get_mut(resource.rep()).unwrap()
            .stream
            .try_get_value_mut()?
            .set(data_type.decode(&value)?)
    }

    async fn destroy(&mut self, resource: Resource<ValueOutput>) -> Result<(), Error> {
        HostValueOutput::drop(self, resource)
    }

    fn drop(&mut self, resource: Resource<ValueOutput>) -> Result<(), Error> {
        if self.outputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}

#[async_trait]
impl HostListOutput for WasmStorage {
    async fn add(&mut self, resource: Resource<ListOutput>, value: Vec<u8>) -> Result<(), Error> {
        let data_type = &self
            .outputs
            .get(resource.rep())?
            .metadata
            .data_type;
        self.outputs
            .get_mut(resource.rep()).unwrap()
            .stream
            .try_get_list_mut()?
            .add(data_type.decode(&value)?)
    }

    async fn pop(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        self.outputs
            .get_mut(resource.rep())?
            .stream
            .try_get_list_mut()?
            .pop()
    }

    async fn clear(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        self.outputs
            .get_mut(resource.rep())?
            .stream
            .try_get_list_mut()?
            .clear()
    }

    async fn set_has_more_rows(
        &mut self,
        resource: Resource<ListOutput>,
        has_more_rows: bool,
    ) -> Result<(), Error> {
        self.outputs
            .get_mut(resource.rep())?
            .stream
            .try_get_list_mut()?
            .set_has_more_rows(has_more_rows)
    }

    async fn destroy(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        HostListOutput::drop(self, resource)
    }

    async fn poll_request(
        &mut self,
        resource: Resource<ListOutput>,
    ) -> Result<Option<ListOutputRequest>, Error> {
        let mut stream = self
            .outputs
            .get(resource.rep())?
            .stream
            .try_get_list()?
            .get_page_request_stream();
        Ok(stream
            .try_recv()
            .map(|page_length| ListOutputRequest::LoadMore(page_length))
            .ok())
    }

    async fn poll_request_blocking(
        &mut self,
        resource: Resource<ListOutput>,
    ) -> Result<ListOutputRequest, Error> {
        let mut stream = self
            .outputs
            .get(resource.rep())?
            .stream
            .try_get_list()?
            .get_page_request_stream();
        let page_length = stream.recv().await?;
        Ok(ListOutputRequest::LoadMore(page_length))
    }

    fn drop(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        if self.outputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}

#[async_trait]
impl HostTreeOutput for WasmStorage {
    async fn add(
        &mut self,
        resource: Resource<TreeOutput>,
        parent: Option<String>,
        nodes: Vec<TreeNode>,
    ) -> Result<(), Error> {
        self.outputs
            .get_mut(resource.rep())?
            .stream
            .try_get_tree_mut()?
            .add(parent, nodes)
    }

    async fn remove(
        &mut self,
        resource: Resource<TreeOutput>,
        parent: String,
    ) -> Result<(), Error> {
        self.outputs
            .get_mut(resource.rep())?
            .stream
            .try_get_tree_mut()?
            .remove(parent)
    }

    async fn clear(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        self.outputs
            .get_mut(resource.rep())?
            .stream
            .try_get_tree_mut()?
            .clear()
    }

    async fn destroy(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        HostTreeOutput::drop(self, resource)
    }

    async fn poll_request(
        &mut self,
        resource: Resource<TreeOutput>,
    ) -> Result<Option<TreeOutputRequest>, Error> {
        let mut stream = self
            .outputs
            .get(resource.rep())?
            .stream
            .try_get_tree()?
            .get_request_children_stream();
        Ok(stream
            .try_recv()
            .map(|parent_id| TreeOutputRequest::LoadChildren(parent_id))
            .ok())
    }

    async fn poll_request_blocking(
        &mut self,
        resource: Resource<TreeOutput>,
    ) -> Result<TreeOutputRequest, Error> {
        let mut stream = self
            .outputs
            .get(resource.rep())?
            .stream
            .try_get_tree()?
            .get_request_children_stream();
        let parent_id = stream.recv().await?;
        Ok(TreeOutputRequest::LoadChildren(parent_id))
    }

    fn drop(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        if self.outputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}
