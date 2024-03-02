use commander::base::types::{
    EnumVariant, Primitive, PrimitiveValue, InputSpec
};
use lazy_static::lazy_static;
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

enum FileEntityType {
    File,
    Directory,
    Symlink,
    Other,
}

impl FileEntityType {
    fn as_commander_data_type() -> Primitive {
        Primitive::EnumType(vec![
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
        ])
    }

    fn to_commander_value(&self) -> PrimitiveValue {
        match self {
            FileEntityType::File => PrimitiveValue::EnumValue(0),
            FileEntityType::Directory => PrimitiveValue::EnumValue(1),
            FileEntityType::Symlink => PrimitiveValue::EnumValue(2),
            FileEntityType::Other => PrimitiveValue::EnumValue(3),
        }
    }
}

lazy_static! {
    static ref OUTPUT_TABLE_COLUMNS: Vec<Column> = vec![
        Column {
            name: "name".to_string(),
            description: "The name of the file".to_string(),
            data_type: Primitive::StringType,
        },
        Column {
            name: "type".to_string(),
            description: "The type of the filesystem entity".to_string(),
            data_type: FileEntityType::as_commander_data_type(),
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
    ];
}

struct ListProgram;

impl Guest for ListProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "List Files".to_string(),
            description: "List files in a directory".to_string(),
            arguments: vec![InputSpec {
                name: "directory".to_string(),
                description: "The top-level directory to list files in".to_string(),
                data_type: DataType::Primitive(Primitive::PathType),
            }],
        }
    }

    fn run(mut inputs: Vec<Value>) -> Result<String, String> {
        let Some(Value::PrimitiveValue(PrimitiveValue::PathValue(path))) = inputs.pop() else {
            Err("Invalid input".to_string())?
        };

        let (base, _) = wasi::filesystem::preopens::get_directories().pop().unwrap();
        let descriptor = ListProgram::navigate_to_dir(base, &path)?;

        let list_output_handle = add_list_output(
            "Files",
            "The list of files",
            &OUTPUT_TABLE_COLUMNS,
        );
        ListProgram::list_files_in_dir(descriptor, list_output_handle)
    }
}

impl ListProgram {
    fn list_files_in_dir(descriptor: Descriptor, output: ListOutput) -> Result<String, String> {
        let entry_stream = wasi::filesystem::types::Descriptor::read_directory(&descriptor)
            .map_err(|code| format!("Error opening directory: {:?}", code))?;
        loop {
            let maybe_entry =
                wasi::filesystem::types::DirectoryEntryStream::read_directory_entry(&entry_stream)
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

            output.add(&Value::TableValue(vec![vec![
                PrimitiveValue::StringValue(file_entry.name),
                PrimitiveValue::NumberValue(file_stat.size as f64),
                ListProgram::file_stat_to_type_enum(&file_stat),
                PrimitiveValue::TimestampValue(
                    file_stat
                        .data_access_timestamp
                        .map(|t| t.seconds * 1000)
                        .unwrap_or(0u64),
                ),
            ]]));
        }
        Ok("Done".to_string())
    }

    fn navigate_to_dir(base: Descriptor, path: &[String]) -> Result<Descriptor, String> {
        if path.is_empty() {
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

    fn file_stat_to_type_enum(stat: &DescriptorStat) -> PrimitiveValue {
        match stat.type_ {
            DescriptorType::RegularFile => FileEntityType::File.to_commander_value(),
            DescriptorType::Directory => FileEntityType::Directory.to_commander_value(),
            DescriptorType::SymbolicLink => FileEntityType::Symlink.to_commander_value(),
            _ => FileEntityType::Other.to_commander_value(),
        }
    }
}
