use wasmtime::component::bindgen;
use wasmtime_wasi::preview2::*;

bindgen!({
    world: "plugin",
    path: "../wit",
    async: true,
    with: {
        "wasi:io/error": bindings::io::error,
        "wasi:io/poll": bindings::io::poll,
        "wasi:io/streams": bindings::io::streams,
        "wasi:clocks/wall-clock": bindings::clocks::wall_clock,
        "wasi:filesystem/types": bindings::filesystem::types,
        "wasi:filesystem/preopens": bindings::filesystem::preopens,
    },
});