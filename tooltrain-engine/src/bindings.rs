pub mod streaming {
    use wasmtime::component::bindgen;
    use wasmtime_wasi::*;
    use wasmtime_wasi_http::bindings::http;

    bindgen!({
        world: "streaming-plugin",
        path: "../wit",
        async: true,
        trappable_imports: true,
        with: {
            "wasi:io/error": bindings::io::error,
            "wasi:io/poll": bindings::io::poll,
            "wasi:io/streams": bindings::io::streams,
            "wasi:clocks/wall-clock": bindings::clocks::wall_clock,
            "wasi:clocks/monotonic-clock": bindings::clocks::monotonic_clock,
            "wasi:filesystem/types": bindings::filesystem::types,
            "wasi:filesystem/preopens": bindings::filesystem::preopens,
            "wasi:http/types": http::types,
            "wasi:http/outgoing-handler": http::outgoing_handler,
        },
    });
}

pub mod discrete {
    use wasmtime::component::bindgen;
    use wasmtime_wasi::*;
    use wasmtime_wasi_http::bindings::http;

    bindgen!({
        world: "discrete-plugin",
        path: "../wit",
        async: true,
        trappable_imports: true,
        with: {
            "tooltrain:base/inputs": super::streaming::tooltrain::base::inputs,
            "wasi:io/error": bindings::io::error,
            "wasi:io/poll": bindings::io::poll,
            "wasi:io/streams": bindings::io::streams,
            "wasi:clocks/wall-clock": bindings::clocks::wall_clock,
            "wasi:clocks/monotonic-clock": bindings::clocks::monotonic_clock,
            "wasi:filesystem/types": bindings::filesystem::types,
            "wasi:filesystem/preopens": bindings::filesystem::preopens,
            "wasi:http/types": http::types,
            "wasi:http/outgoing-handler": http::outgoing_handler,
        },
    });
}

pub use streaming::tooltrain::base::inputs;
pub use streaming::tooltrain::base::streaming_inputs;
pub use streaming::tooltrain::base::streaming_outputs;
