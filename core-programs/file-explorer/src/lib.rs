use std::{
    ffi::OsStr,
    fs,
    path::{Component, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Error};
use commander_data::{CommanderCoder, CommanderPathDataType};
use commander_rust_guest::{
    add_tree_output,
    commander::base::{
        inputs::ArgumentSpec,
        streaming_inputs::Input,
        streaming_outputs::{TreeNode, TreeOutput, TreeOutputRequest},
    },
    export_guest, Guest, Schema,
};
use tokio::{runtime, sync::RwLock, task::JoinHandle};
use tokio_stream::StreamExt;

struct FileExplorerProgram;

impl Guest for FileExplorerProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "File Explorer".to_string(),
            description: "Outputs a tree of files and directories".to_string(),
            arguments: vec![ArgumentSpec {
                name: "root".to_string(),
                description: "The root directory for the file tree".to_string(),
                data_type: CommanderPathDataType {}.type_string(),
                supports_updates: true,
            }],
            performs_state_change: false,
        }
    }

    fn run(inputs: Vec<Input>) -> Result<String, String> {
        let runtime = runtime::Builder::new_current_thread()
            .build()
            .map_err(|e| e.to_string())?;
        let result = runtime.block_on(run_internal(inputs));
        result.map_err(|e| e.to_string())
    }
}

async fn run_internal(inputs: Vec<Input>) -> Result<String, Error> {
    let Input::ValueInput(path_input) = &inputs[0] else {
        return Err(anyhow!("First input is not a value"));
    };

    let tree_output = Arc::new(RwLock::new(add_tree_output(
        "Tree",
        "A tree of files and directories starting at the specified root",
        "path",
    )));

    let mut running_job: Option<JoinHandle<()>> = None;
    while let Some(Some(path_value)) = path_input.values(CommanderPathDataType {}).next().await {
        if let Some(job) = running_job {
            job.abort();
        }

        tree_output.write().await.clear();

        let cloned_tree_output = tree_output.clone();
        running_job = Some(tokio::spawn(async move {
            let explorer = FileExplorer {
                root: path_value,
                output: cloned_tree_output,
            };
            explorer.run().await;
        }));
    }

    Ok("Done".to_string())
}

struct FileExplorer {
    root: PathBuf,
    output: Arc<RwLock<TreeOutput>>,
}

impl FileExplorer {
    async fn run(&self) {
        self.add_paths(vec![]).await;

        while let Some(tree_update) = self.output.write().await.next().await {
            match tree_update {
                TreeOutputRequest::LoadChildren(parent_id) => {
                    let relative_path: Vec<&str> = parent_id.split('/').collect();
                    self.add_paths(relative_path).await;
                }
                TreeOutputRequest::Close => break,
            }
        }
    }

    async fn add_paths(&self, relative_path: Vec<&str>) {
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
                value: CommanderPathDataType {}
                    .encode(
                        full_pathbuf
                            .clone()
                            .join(entry.file_name())
                            .components()
                            .map(Component::as_os_str)
                            .map(OsStr::to_string_lossy)
                            .map(String::from)
                            .collect(),
                    )
                    .unwrap(),
            })
            .collect();

        self.output
            .write()
            .await
            .add(parent_node_id.as_deref(), &children);
    }

    fn validate_relative_path(relative_path: &[&str]) -> bool {
        relative_path
            .iter()
            .all(|component| *component != ".." && !component.contains('/'))
    }
}

export_guest!(FileExplorerProgram);
