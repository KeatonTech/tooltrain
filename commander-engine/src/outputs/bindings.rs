use crate::{
    bindings::{
        commander::base::outputs::{
            HostListOutput, HostTreeOutput, HostValueOutput, ListOutputRequest, TreeNode,
            TreeOutputRequest,
        },
        Column, DataType, ListOutput, PluginImports, TreeOutput, ValueOutput,
    },
    storage::WasmStorage,
    Value,
};

use anyhow::{anyhow, Error};
use async_trait::async_trait;

use wasmtime::component::*;

#[async_trait]
impl PluginImports for WasmStorage {
    async fn add_value_output(
        &mut self,
        name: String,
        description: String,
        data_type: DataType,
        initial_value: Option<Value>,
    ) -> Result<Resource<ValueOutput>, Error> {
        Ok(Resource::new_own(self.outputs.add_value_output(
            name,
            description,
            data_type,
            initial_value,
        )?))
    }

    async fn add_list_output(
        &mut self,
        name: String,
        description: String,
        columns: Vec<Column>,
    ) -> Result<Resource<ListOutput>, Error> {
        Ok(Resource::new_own(self.outputs.add_list_output(
            name,
            description,
            columns,
        )?))
    }

    async fn add_tree_output(
        &mut self,
        name: String,
        description: String,
        data_type: DataType,
    ) -> Result<Resource<TreeOutput>, Error> {
        Ok(Resource::new_own(self.outputs.add_tree_output(
            name,
            description,
            data_type,
        )?))
    }
}

#[async_trait]
impl HostValueOutput for WasmStorage {
    async fn set(&mut self, resource: Resource<ValueOutput>, value: Value) -> Result<(), Error> {
        self.outputs
            .get_output_mut(resource.rep())?
            .stream
            .try_get_value_mut()?
            .set(value)
    }

    async fn destroy(&mut self, resource: Resource<ValueOutput>) -> Result<(), Error> {
        HostValueOutput::drop(self, resource)
    }

    fn drop(&mut self, resource: Resource<ValueOutput>) -> Result<(), Error> {
        if self.outputs.remove_output(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}

#[async_trait]
impl HostListOutput for WasmStorage {
    async fn add(&mut self, resource: Resource<ListOutput>, value: Value) -> Result<(), Error> {
        self.outputs
            .get_output_mut(resource.rep())?
            .stream
            .try_get_list_mut()?
            .add(value)
    }

    async fn pop(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        self.outputs
            .get_output_mut(resource.rep())?
            .stream
            .try_get_list_mut()?
            .pop()
    }

    async fn clear(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        self.outputs
            .get_output_mut(resource.rep())?
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
            .get_output_mut(resource.rep())?
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
    ) -> Result<ListOutputRequest, Error> {
        let mut stream = self
            .outputs
            .get_output(resource.rep())?
            .stream
            .try_get_list()?
            .get_page_request_stream();
        let page_length = stream.recv().await?;
        Ok(ListOutputRequest::LoadMore(page_length))
    }

    fn drop(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        if self.outputs.remove_output(resource.rep())? {
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
            .get_output_mut(resource.rep())?
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
            .get_output_mut(resource.rep())?
            .stream
            .try_get_tree_mut()?
            .remove(parent)
    }

    async fn clear(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        self.outputs
            .get_output_mut(resource.rep())?
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
    ) -> Result<TreeOutputRequest, Error> {
        let mut stream = self
            .outputs
            .get_output(resource.rep())?
            .stream
            .try_get_tree()?
            .get_request_children_stream();
        let parent_id = stream.recv().await?;
        Ok(TreeOutputRequest::LoadChildren(parent_id))
    }

    fn drop(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        if self.outputs.remove_output(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}
