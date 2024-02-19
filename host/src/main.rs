use anyhow::Error;
use commander_engine::{CommanderEngine, PrimitiveValue, ProgramSource, Value};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let engine = CommanderEngine::new();
    let ls_program_source = ProgramSource::FilePath(
        std::path::Path::new(
            "/Users/keatonbrandt/Documents/Development/Rust/commander/target/wasm32-wasi/debug/ls.wasm"
        ).to_owned(),
    );
    let mut ls_program = engine.open_program(ls_program_source).await?;
    let mut run = ls_program
        .run(vec![Value::PrimitiveValue(PrimitiveValue::PathValue(
            vec![
                "Users".to_string(),
                "keatonbrandt".to_string(),
                "Documents".to_string(),
                "Development".to_string(),
            ],
        ))])
        .await?;
    let result = run.get_result().await;
    println!("Final result: {:?}", result);
    println!("Outputs: {:?}", run.outputs_snapshot());
    Ok(())
}
