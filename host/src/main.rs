use anyhow::{anyhow, Error};
use commander_engine::{
    CommanderEngine, OutputChange, OutputHandle, Outputs, PrimitiveValue, ProgramSource,
    TreeOutputHandle, Value,
};

use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let engine = CommanderEngine::new();
    let file_explorer_program_source = ProgramSource::FilePath(
        std::path::Path::new("./target/wasm32-wasi/debug/file_explorer.wasm").to_owned(),
    );
    let mut file_explorer_program = engine.open_program(file_explorer_program_source).await?;
    let mut run = file_explorer_program
        .run(vec![Value::PrimitiveValue(PrimitiveValue::PathValue(
            vec!["Users".to_string()],
        ))])
        .await?;

    let tree_output = get_tree_output(run.outputs()).await?;
    tokio::spawn(listen_for_tree_changes(tree_output.clone()));

    println!("Enter directories to inspect then press enter.");
    let mut input_stream = ReaderStream::new(tokio::io::stdin())
        .take_while(|r| r.is_ok())
        .filter_map(Result::ok)
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string());

    while let Some(path) = input_stream.next().await {
        println!("Opening {}", path);
        tree_output.request_children(path)?;
    }

    let result = run.get_result().await;
    println!("Final result: {:?}", result);
    println!("Outputs: {:?}", run.outputs().snapshot_output_values());
    Ok(())
}

async fn get_tree_output(outputs: Outputs) -> Result<TreeOutputHandle, Error> {
    let mut stream = outputs.updates();
    while let Some(output_change) = stream.next().await {
        println!("Received an output change: {:?}", output_change);
        match output_change {
            OutputChange::Added(handle) => match handle {
                OutputHandle::Tree(t) => return Ok(t),
                _ => println!("Unsupported output type: {:?}", handle.metadata().data_type),
            },
            OutputChange::Removed(_) => todo!(),
        }
    }
    Err(anyhow!("Tree output was never added"))
}

async fn listen_for_tree_changes(handle: TreeOutputHandle) {
    let mut stream = handle.values().unwrap();
    while let Some(tree_change) = stream.next().await {
        println!("Received tree change: {:?}", tree_change);
    }
}
