use anyhow::Error;
use commander_engine::{CommanderEngine, OutputChange, OutputId, Outputs, PrimitiveValue, ProgramSource, Value};
use tokio_stream::{StreamExt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let engine = CommanderEngine::new();
    let mastodon_program_source = ProgramSource::FilePath(
        std::path::Path::new(
            "/Users/keatonbrandt/Documents/Development/Rust/commander/target/wasm32-wasi/debug/mastodon_feed.wasm"
        ).to_owned(),
    );
    let mut mastodon_program = engine.open_program(mastodon_program_source).await?;
    let mut run = mastodon_program
        .run(vec![Value::PrimitiveValue(PrimitiveValue::StringValue("me.dm".to_string()))])
        .await?;

    tokio::spawn(listen_for_output_changes(run.outputs()));

    let result = run.get_result().await;
    println!("Final result: {:?}", result);
    println!("Outputs: {:?}", run.outputs().snapshot_output_values());
    Ok(())
}

async fn listen_for_output_changes(outputs: Outputs) {
    let mut stream = outputs.outputs_list_change_stream();
    while let Some(output_change) = stream.next().await {
        println!("Received an output change: {:?}", output_change);
        match output_change {
            OutputChange::Added(metadata) => {
                tokio::spawn(listen_for_list_changes(outputs.clone(), metadata.id));
            }
            _ => ()
        }
    }
    println!("Outputs ended");
}

async fn listen_for_list_changes(outputs: Outputs, id: OutputId) {
    let mut stream = outputs.list_output_changes_stream(id).unwrap();
    while let Some(list_change) = stream.next().await {
        println!("Received list item: {:?}", list_change);
    }
}