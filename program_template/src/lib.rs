use commander::base::types::{
    Column, EnumVariant, Primitive, PrimitiveValue, StreamSpec, ValueEvent,
};

wit_bindgen::generate!({
    path: "../wit",
    world: "plugin",
    exports: {
        world: ProgramNameProgram,
    },
});

struct ProgramNameProgram;

impl Guest for ProgramNameProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "TODO".to_string(),
            description: "TODO".to_string(),
            arguments: vec![],
        }
    }

    fn run(mut inputs: Vec<Value>) -> Result<String, String> {
        todo!("Implement me!");
    }
}
