#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{
    fs::{self, FileType},
    path::PathBuf,
};

use serde::Serialize;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_files_in_relative_directory])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Serialize)]
enum FileEntry {
    File { name: String, path: String },
    Directory { name: String, path: String },
}

#[tauri::command]
fn list_files_in_relative_directory(path: PathBuf) -> Vec<FileEntry> {
    fs::read_dir(path)
        .unwrap()
        .filter_map(|entry_result| {
            let Ok(dir_entry) = entry_result else {
                return None;
            };
            let Ok(file_type) = dir_entry.file_type() else {
                return None;
            };
            if file_type.is_dir() {
                Some(FileEntry::Directory {
                    name: dir_entry.file_name().to_string_lossy().to_string(),
                    path: dir_entry.path().to_string_lossy().to_string(),
                })
            } else if file_type.is_file() {
                Some(FileEntry::File {
                    name: dir_entry.file_name().to_string_lossy().to_string(),
                    path: dir_entry.path().to_string_lossy().to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}
