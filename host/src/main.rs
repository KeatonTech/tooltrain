use anyhow::Error;
use commander_engine::{CommanderEngine, PrimitiveValue, ProgramSource, Value};

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
    let result = run.get_result().await;
    println!("Final result: {:?}", result);
    println!("Outputs: {:?}", run.outputs_snapshot());
    Ok(())
}
