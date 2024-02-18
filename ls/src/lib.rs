use commander::base::types::{
    Column, DataType, EnumVariant, Primitive, PrimitiveValue, StreamSpec, ValueEvent,
};
use wasi::filesystem::types::{
    Descriptor, DescriptorFlags, DescriptorStat, DescriptorType, OpenFlags, PathFlags,
};

wit_bindgen::generate!({
    path: "../wit",
    world: "plugin",
    exports: {
        world: ListProgram,
    },
});

struct ListProgram;

impl Guest for ListProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "List Files".to_string(),
            description: "List files in a directory".to_string(),
            arguments: vec![StreamSpec {
                name: "directory".to_string(),
                description: "The directory to list files in".to_string(),
                data_type: DataType::Primitive(Primitive::PathType),
            }],
            outputs: vec![StreamSpec {
                name: "files".to_string(),
                description: "The files in the directory".to_string(),
                data_type: DataType::TableType(vec![
                    Column {
                        name: "name".to_string(),
                        description: "The name of the file".to_string(),
                        data_type: Primitive::StringType,
                    },
                    Column {
                        name: "type".to_string(),
                        description: "The type of the filesystem entity".to_string(),
                        data_type: Primitive::EnumType(vec![
                            EnumVariant {
                                name: "file".to_string(),
                                description: "A regular file".to_string(),
                            },
                            EnumVariant {
                                name: "directory".to_string(),
                                description: "A directory".to_string(),
                            },
                            EnumVariant {
                                name: "symlink".to_string(),
                                description: "A symbolic link".to_string(),
                            },
                            EnumVariant {
                                name: "other".to_string(),
                                description: "Some other kind of entity not represented here".to_string(),
                            },
                        ]),
                    },
                    Column {
                        name: "size".to_string(),
                        description: "The size of the file, in bytes".to_string(),
                        data_type: Primitive::NumberType,
                    },
                    Column {
                        name: "accessed_at".to_string(),
                        description: "The time when the file was last accessed".to_string(),
                        data_type: Primitive::TimestampType,
                    },
                ]),
            }],
        }
    }

    fn run(mut inputs: Vec<Value>, outputs: Vec<OutputEventsStream>) -> Result<String, String> {
        if let Some(Value::PrimitiveValue(PrimitiveValue::PathValue(path))) = inputs.pop() {
            let (base, base_path) = wasi::filesystem::preopens::get_directories().pop().unwrap();
            println!("Base Path: {}", base_path);
            let descriptor = ListProgram::navigate_to_dir(base, &path)?;
            let entry_stream = wasi::filesystem::types::Descriptor::read_directory(&descriptor)
                .map_err(|code| format!("Error opening directory: {:?}", code))?;
            loop {
                let maybe_entry =
                    wasi::filesystem::types::DirectoryEntryStream::read_directory_entry(
                        &entry_stream,
                    )
                    .map_err(|code| format!("Error reading directory: {:?}", code))?;
                if maybe_entry.is_none() {
                    break;
                }
                let file_entry = maybe_entry.unwrap();
                let file_stat = wasi::filesystem::types::Descriptor::stat_at(
                    &descriptor,
                    PathFlags::SYMLINK_FOLLOW,
                    &file_entry.name,
                )
                .map_err(|code| format!("Error reading {} (code: {code})", file_entry.name))?;

                outputs[0].send(&ValueEvent::Add(Value::TableValue(vec![vec![
                    PrimitiveValue::StringValue(file_entry.name),
                    PrimitiveValue::NumberValue(file_stat.size as f64),
                    PrimitiveValue::EnumValue(ListProgram::file_stat_to_type_enum(&file_stat)),
                    PrimitiveValue::TimestampValue(
                        file_stat
                            .data_access_timestamp
                            .map(|t| t.seconds * 1000)
                            .unwrap_or(0u64),
                    ),
                ]])));
            }
            Ok("Done".to_string())
        } else {
            Err("Invalid input".to_string())
        }
    }
}

impl ListProgram {
    fn navigate_to_dir(base: Descriptor, path: &[String]) -> Result<Descriptor, String> {
        if path.len() == 0 {
            return Ok(base);
        }
        let next_dir = wasi::filesystem::types::Descriptor::open_at(
            &base,
            PathFlags::SYMLINK_FOLLOW,
            &path[0],
            OpenFlags::DIRECTORY,
            DescriptorFlags::READ,
        )
        .map_err(|code| format!("Could not open directory {} (code {code})", path[0]))?;
        ListProgram::navigate_to_dir(next_dir, &path[1..])
    }

    fn file_stat_to_type_enum(stat: &DescriptorStat) -> u16 {
        match stat.type_ {
            DescriptorType::RegularFile => 0,
            DescriptorType::Directory => 1,
            DescriptorType::SymbolicLink => 2,
            _ => 3,
        }
    }
}
