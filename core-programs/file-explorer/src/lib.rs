use std::{
    ffi::OsStr,
    fs::{self},
    path::{Component, PathBuf},
};

use commander::base::{
    outputs::{TreeNode, TreeOutputRequest},
    types::{InputSpec, Primitive, PrimitiveValue},
};

wit_bindgen::generate!({
    path: "../../wit",
    world: "plugin",
});

struct FileExplorerProgram;

impl Guest for FileExplorerProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "File Explorer".to_string(),
            description: "Outputs a tree of files and directories".to_string(),
            arguments: vec![InputSpec {
                name: "root".to_string(),
                description: "The root directory for the file tree".to_string(),
                data_type: DataType::Primitive(Primitive::PathType),
            }],
        }
    }

    fn run(mut inputs: Vec<Value>) -> Result<String, String> {
        let Some(Value::PrimitiveValue(PrimitiveValue::PathValue(path))) = inputs.pop() else {
            return Err("Malformed arguments".to_string());
        };

        let tree_output = add_tree_output(
            "Tree",
            "A tree of files and directories starting at the specified root",
            &DataType::Primitive(Primitive::PathType),
        );
        let explorer = FileExplorer {
            root: PathBuf::from_iter(path),
            output: tree_output,
        };
        explorer.run();

        Ok("Done".to_string())
    }
}

struct FileExplorer {
    root: PathBuf,
    output: TreeOutput,
}

impl FileExplorer {
    fn run(&self) {
        self.add_paths(vec![]);

        while let TreeOutputRequest::LoadChildren(parent_id) = self.output.poll_request() {
            let relative_path: Vec<&str> = parent_id.split('/').collect();
            self.add_paths(relative_path);
        }
    }

    fn add_paths(&self, relative_path: Vec<&str>) {
        if !FileExplorer::validate_relative_path(&relative_path) {
            eprintln!("Invalid relative path: {}", relative_path.join("/"));
            return;
        }

        let relative_pathbuf = PathBuf::from_iter(relative_path.clone());
        let full_pathbuf = self.root.join(relative_pathbuf.clone());

        let Ok(dir) = fs::read_dir(full_pathbuf.clone()) else {
            eprintln!(
                "Directory does not exist: {}",
                full_pathbuf.to_string_lossy()
            );
            return;
        };

        let parent_node_id = if relative_path.is_empty() {
            None
        } else {
            Some(relative_pathbuf.clone().to_string_lossy().to_string())
        };

        let children: Vec<TreeNode> = dir
            .filter_map(Result::ok)
            .map(|entry| TreeNode {
                id: relative_pathbuf
                    .clone()
                    .join(entry.file_name())
                    .to_string_lossy()
                    .to_string(),
                has_children: entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
                value: Value::PrimitiveValue(PrimitiveValue::PathValue(
                    full_pathbuf
                        .clone()
                        .join(entry.file_name())
                        .components()
                        .map(Component::as_os_str)
                        .map(OsStr::to_string_lossy)
                        .map(String::from)
                        .collect(),
                )),
            })
            .collect();

        self.output.add(parent_node_id.as_deref(), &children);
    }

    fn validate_relative_path(relative_path: &[&str]) -> bool {
        relative_path
            .iter()
            .all(|component| *component != ".." && !component.contains('/'))
    }
}

export!(FileExplorerProgram);
