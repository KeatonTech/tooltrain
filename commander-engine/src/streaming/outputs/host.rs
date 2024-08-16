use crate::{
    bindings::{
        inputs::TreeNode,
        streaming::{ListOutput, TreeOutput, ValueOutput},
        streaming_outputs::{
            HostListOutput, HostListOutputRequestStream, HostTreeOutput,
            HostTreeOutputRequestStream, HostValueOutput, ListOutputRequest,
            ListOutputRequestStream, TreeOutputRequest, TreeOutputRequestStream,
        },
    },
    streaming::storage::WasmStorage,
};

use anyhow::{anyhow, Error};
use async_trait::async_trait;

use commander_data::CommanderCoder;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use wasmtime::component::*;
use wasmtime_wasi::WasiImpl;

#[async_trait]
impl HostValueOutput for WasiImpl<&mut WasmStorage> {
    async fn set(&mut self, resource: Resource<ValueOutput>, value: Vec<u8>) -> Result<(), Error> {
        let data_type = &self.0.outputs.get(resource.rep())?.metadata.data_type;
        self.0
            .outputs
            .get(resource.rep())
            .unwrap()
            .stream
            .write()
            .try_get_value_mut()?
            .set(data_type.decode(&value)?)
    }

    async fn destroy(&mut self, resource: Resource<ValueOutput>) -> Result<(), Error> {
        HostValueOutput::drop(self, resource)
    }

    fn drop(&mut self, resource: Resource<ValueOutput>) -> Result<(), Error> {
        if self.0.outputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}

#[async_trait]
impl HostListOutput for WasiImpl<&mut WasmStorage> {
    async fn add(&mut self, resource: Resource<ListOutput>, value: Vec<u8>) -> Result<(), Error> {
        let data_type = &self.0.outputs.get(resource.rep())?.metadata.data_type;
        self.0
            .outputs
            .get(resource.rep())
            .unwrap()
            .stream
            .write()
            .try_get_list_mut()?
            .add(data_type.decode(&value)?)
    }

    async fn pop(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        self.0
            .outputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_list_mut()?
            .pop()
    }

    async fn clear(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        self.0
            .outputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_list_mut()?
            .clear()
    }

    async fn set_has_more_rows(
        &mut self,
        resource: Resource<ListOutput>,
        has_more_rows: bool,
    ) -> Result<(), Error> {
        self.0
            .outputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_list_mut()?
            .set_has_more_rows(has_more_rows)
    }

    async fn destroy(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        HostListOutput::drop(self, resource)
    }

    async fn get_request_stream(
        &mut self,
        resource: Resource<ListOutput>,
    ) -> Result<Resource<ListOutputRequestStream>, Error> {
        Ok(Resource::new_own(
            self.0
                .output_request_streams
                .list_request_streams
                .add_stream(
                    BroadcastStream::new(
                        self.0
                            .outputs
                            .get(resource.rep())?
                            .stream
                            .read()
                            .try_get_list()?
                            .get_page_request_stream(),
                    )
                    .map(|request_result| match request_result {
                        Ok(count) => ListOutputRequest::LoadMore(count),
                        Err(_) => ListOutputRequest::Close,
                    }),
                ),
        ))
    }

    fn drop(&mut self, resource: Resource<ListOutput>) -> Result<(), Error> {
        if self.0.outputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}

#[async_trait]
impl HostTreeOutput for WasiImpl<&mut WasmStorage> {
    async fn add(
        &mut self,
        resource: Resource<TreeOutput>,
        parent: Option<String>,
        nodes: Vec<TreeNode>,
    ) -> Result<(), Error> {
        self.0
            .outputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_tree_mut()?
            .add(parent, nodes)
    }

    async fn remove(
        &mut self,
        resource: Resource<TreeOutput>,
        parent: String,
    ) -> Result<(), Error> {
        self.0
            .outputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_tree_mut()?
            .remove(parent)
    }

    async fn clear(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        self.0
            .outputs
            .get(resource.rep())?
            .stream
            .write()
            .try_get_tree_mut()?
            .clear()
    }

    async fn destroy(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        HostTreeOutput::drop(self, resource)
    }

    async fn get_request_stream(
        &mut self,
        resource: Resource<TreeOutput>,
    ) -> Result<Resource<TreeOutputRequestStream>, Error> {
        Ok(Resource::new_own(
            self.0
                .output_request_streams
                .tree_request_streams
                .add_stream(
                    BroadcastStream::new(
                        self.0
                            .outputs
                            .get(resource.rep())?
                            .stream
                            .write()
                            .try_get_tree_mut()?
                            .get_request_children_stream(),
                    )
                    .map(|request_result| match request_result {
                        Ok(parent) => TreeOutputRequest::LoadChildren(parent),
                        Err(_) => TreeOutputRequest::Close,
                    }),
                ),
        ))
    }

    fn drop(&mut self, resource: Resource<TreeOutput>) -> Result<(), Error> {
        if self.0.outputs.remove(resource.rep())? {
            Ok(())
        } else {
            Err(anyhow!("Could not destroy non-existent output"))
        }
    }
}

#[async_trait]
impl HostListOutputRequestStream for WasiImpl<&mut WasmStorage> {
    async fn poll_request(
        &mut self,
        resource: Resource<ListOutputRequestStream>,
    ) -> Result<Option<ListOutputRequest>, Error> {
        self.0
            .output_request_streams
            .list_request_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Output request stream not found"))?
            .poll_request()
    }

    async fn poll_request_blocking(
        &mut self,
        resource: Resource<ListOutputRequestStream>,
    ) -> Result<ListOutputRequest, Error> {
        self.0
            .output_request_streams
            .list_request_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Output request stream not found"))?
            .poll_request_blocking()
            .await
    }

    fn drop(&mut self, resource: Resource<ListOutputRequestStream>) -> Result<(), Error> {
        if self
            .0
            .output_request_streams
            .list_request_streams
            .remove(resource.rep())
        {
            Ok(())
        } else {
            Err(anyhow!(
                "Could not destroy non-existent output request stream"
            ))
        }
    }
}

#[async_trait]
impl HostTreeOutputRequestStream for WasiImpl<&mut WasmStorage> {
    async fn poll_request(
        &mut self,
        resource: Resource<TreeOutputRequestStream>,
    ) -> Result<Option<TreeOutputRequest>, Error> {
        self.0
            .output_request_streams
            .tree_request_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Output request stream not found"))?
            .poll_request()
    }

    async fn poll_request_blocking(
        &mut self,
        resource: Resource<TreeOutputRequestStream>,
    ) -> Result<TreeOutputRequest, Error> {
        self.0
            .output_request_streams
            .tree_request_streams
            .get_mut(resource.rep())
            .ok_or_else(|| anyhow!("Output request stream not found"))?
            .poll_request_blocking()
            .await
    }

    fn drop(&mut self, resource: Resource<TreeOutputRequestStream>) -> Result<(), Error> {
        if self
            .0
            .output_request_streams
            .list_request_streams
            .remove(resource.rep())
        {
            Ok(())
        } else {
            Err(anyhow!(
                "Could not destroy non-existent output request stream"
            ))
        }
    }
}
