use commander_data::{
    CommanderCoder, CommanderEnumDataType, CommanderNumberDataType, CommanderPathDataType,
    CommanderStringDataType, CommanderStructDataType, CommanderStructTypeBuilder, CommanderValue,
};
use commander_rust_guest::{
    add_list_output,
    commander::base::{inputs::ArgumentSpec, streaming_inputs::Input},
    export_guest,
    wasi::{
        self,
        filesystem::types::{
            Descriptor, DescriptorFlags, DescriptorStat, DescriptorType, OpenFlags, PathFlags,
        },
    },
    Guest, ListOutput, Schema,
};
use maplit::btreemap;
use once_cell::sync::Lazy;

static FILE_ENTITY_TYPE: Lazy<CommanderEnumDataType> = Lazy::new(|| {
    CommanderEnumDataType::new(
        "FileEntityType".to_string(),
        vec![
            "FILE".to_string(),
            "DIRECTORY".to_string(),
            "SYMLINK".to_string(),
            "OTHER".to_string(),
        ],
    )
});

static FILE_STRUCT: Lazy<CommanderStructDataType> = Lazy::new(|| {
    CommanderStructTypeBuilder::new("File")
        .add_field("name", CommanderStringDataType {})
        .add_field("size", CommanderNumberDataType {})
        .add_field("type", FILE_ENTITY_TYPE.clone())
        .build()
});

enum FileEntityType {
    File,
    Directory,
    Symlink,
    Other,
}

impl FileEntityType {
    fn to_commander_value(&self) -> CommanderValue {
        match self {
            FileEntityType::File => FILE_ENTITY_TYPE.get_variant("FILE").unwrap().into(),
            FileEntityType::Directory => FILE_ENTITY_TYPE.get_variant("DIRECTORY").unwrap().into(),
            FileEntityType::Symlink => FILE_ENTITY_TYPE.get_variant("SYMLINK").unwrap().into(),
            FileEntityType::Other => FILE_ENTITY_TYPE.get_variant("OTHER").unwrap().into(),
        }
    }
}

struct ListProgram;

impl Guest for ListProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "List Files".to_string(),
            description: "List files in a directory".to_string(),
            arguments: vec![ArgumentSpec {
                name: "directory".to_string(),
                description: "The top-level directory to list files in".to_string(),
                data_type: CommanderPathDataType {}.type_string(),
                supports_updates: false,
            }],
            performs_state_change: false,
        }
    }

    fn run(inputs: Vec<Input>) -> Result<String, String> {
        let Some(Input::ValueInput(path)) = &inputs.first() else {
            return Err("Invalid input".to_string());
        };
        let pathbuf = CommanderPathDataType {}
            .decode(&path.get().unwrap())
            .map_err(|_| "Could not read path".to_string())?;
        let path_components: Vec<String> = pathbuf
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();

        let (base, _) = wasi::filesystem::preopens::get_directories().pop().unwrap();
        let descriptor = ListProgram::navigate_to_dir(base, &path_components)?;

        let list_output_handle =
            add_list_output("Files", "The list of files", &FILE_STRUCT.type_string());
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

            output.add(
                &FILE_STRUCT
                    .encode(btreemap! {
                        "name".to_string() => file_entry.name.into(),
                        "size".to_string() => (file_stat.size as f64).into(),
                        "type".to_string() => ListProgram::file_stat_to_type_enum(&file_stat),
                    })
                    .unwrap(),
            );
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

    fn file_stat_to_type_enum(stat: &DescriptorStat) -> CommanderValue {
        match stat.type_ {
            DescriptorType::RegularFile => FileEntityType::File.to_commander_value(),
            DescriptorType::Directory => FileEntityType::Directory.to_commander_value(),
            DescriptorType::SymbolicLink => FileEntityType::Symlink.to_commander_value(),
            _ => FileEntityType::Other.to_commander_value(),
        }
    }
}

export_guest!(ListProgram);
