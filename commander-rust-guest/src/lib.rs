use commander::base::streaming_outputs::{ListOutputRequest, TreeOutputRequest};
use commander_data::CommanderCoder;
use std::task::Poll;
use tokio_stream::{once, Stream, StreamExt};

wit_bindgen::generate!({
    path: "../wit",
    world: "streaming-plugin",
});

pub use commander::base::streaming_inputs::{ListChange, TreeChange};
pub use commander::base::streaming_outputs::TreeNode;

#[macro_export]
macro_rules! export_guest {
    ($i:ty) => {
        const _: () = {
            #[export_name = "get-schema"]
            unsafe extern "C" fn export_get_schema() -> *mut u8 {
                commander_rust_guest::_export_get_schema_cabi::<$i>()
            }
            #[export_name = "cabi_post_get-schema"]
            unsafe extern "C" fn _post_return_get_schema(arg0: *mut u8) {
                commander_rust_guest::__post_return_get_schema::<$i>(arg0)
            }
            #[export_name = "run"]
            unsafe extern "C" fn export_run(arg0: *mut u8, arg1: usize) -> *mut u8 {
                commander_rust_guest::_export_run_cabi::<$i>(arg0, arg1)
            }
            #[export_name = "cabi_post_run"]
            unsafe extern "C" fn _post_return_run(arg0: *mut u8) {
                commander_rust_guest::__post_return_run::<$i>(arg0)
            }
        };
    };
}

impl Stream for &ValueInput {
    type Item = Option<Vec<u8>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.poll_change() {
            Some(change) => std::task::Poll::Ready(Some(change)),
            None => std::task::Poll::Pending,
        }
    }
}

impl ValueInput {
    pub fn values<DT: CommanderCoder + 'static>(
        &self,
        data_type: DT,
    ) -> impl Stream<Item = Option<DT::Value>> + '_ {
        once(self.get())
            .chain(self)
            .map(move |data| data.map(|bytes| data_type.decode(&bytes).unwrap()))
    }
}

impl Stream for ListInput {
    type Item = ListChange;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.poll_change() {
            Some(change) => std::task::Poll::Ready(Some(change)),
            None => std::task::Poll::Pending,
        }
    }
}

impl Stream for TreeInput {
    type Item = TreeChange;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.poll_change() {
            Some(change) => std::task::Poll::Ready(Some(change)),
            None => std::task::Poll::Pending,
        }
    }
}

impl Stream for ListOutput {
    type Item = ListOutputRequest;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.poll_request() {
            Some(change) => std::task::Poll::Ready(Some(change)),
            None => std::task::Poll::Pending,
        }
    }
}

impl Stream for TreeOutput {
    type Item = TreeOutputRequest;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.poll_request() {
            Some(change) => std::task::Poll::Ready(Some(change)),
            None => std::task::Poll::Pending,
        }
    }
}
