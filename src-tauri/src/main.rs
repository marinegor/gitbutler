mod deltas;
mod fs;
mod projects;
mod sessions;
mod storage;
mod watchers;

use deltas::Delta;
use fs::list_files;
use git2::Repository;
use log;
use projects::Project;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use storage::Storage;
use tauri::{InvokeError, Manager, Runtime, State, Window};
use tauri_plugin_log::{
    fern::colors::{Color, ColoredLevelConfig},
    LogTarget,
};
use watchers::WatcherCollection;

struct AppState {
    watchers: WatcherCollection,
    projects_storage: projects::Storage,
}

#[tauri::command]
fn list_project_files(state: State<'_, AppState>, project_id: &str) -> Result<Vec<String>, Error> {
    if let Some(project) = state.projects_storage.get_project(project_id)? {
        let project_path = Path::new(&project.path);
        let repo = match Repository::open(project_path) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to open: {}", e),
        };
        let files = list_files(project_path)?;
        let meta_commit = watchers::get_meta_commit(&repo);
        let tree = meta_commit.tree().unwrap();
        let non_ignored_files: Vec<String> = files
            .iter()
            .filter_map(|file| {
                let file_path = Path::new(file);
                if let Ok(_object) = tree.get_path(file_path) {
                    Some(file.to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(non_ignored_files)
    } else {
        Err("Project not found".into())
    }
}

#[tauri::command]
fn read_project_file(
    state: State<'_, AppState>,
    project_id: &str,
    file_path: &str,
) -> Result<Option<String>, InvokeError> {
    if let Some(project) = state.projects_storage.get_project(project_id)? {
        let project_path = Path::new(&project.path);
        let repo = match Repository::open(project_path) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to open: {}", e),
        };
        let meta_commit = watchers::get_meta_commit(&repo);
        let tree = meta_commit.tree().unwrap();
        if let Ok(object) = tree.get_path(Path::new(&file_path)) {
            let blob = object.to_object(&repo).unwrap().into_blob().unwrap();
            let contents = String::from_utf8(blob.content().to_vec()).unwrap();
            Ok(Some(contents))
        } else {
            Ok(None)
        }
    } else {
        Err("Project not found".into())
    }
}

#[tauri::command]
fn add_project<R: Runtime>(
    window: Window<R>,
    state: State<'_, AppState>,
    path: &str,
) -> Result<Project, InvokeError> {
    for project in state.projects_storage.list_projects()? {
        if project.path == path {
            return Err("Project already exists".into());
        }
    }

    let project = projects::Project::from_path(path.to_string());
    if project.is_ok() {
        let project = project.unwrap();
        state.projects_storage.add_project(&project)?;
        watchers::watch(window, &state.watchers, &project)?;
        return Ok(project);
    } else {
        return Err(project.err().unwrap().into());
    }
}

#[tauri::command]
fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, InvokeError> {
    state.projects_storage.list_projects().map_err(|e| e.into())
}

#[tauri::command]
fn delete_project(state: State<'_, AppState>, id: &str) -> Result<(), InvokeError> {
    if let Some(project) = state.projects_storage.get_project(id)? {
        watchers::unwatch(&state.watchers, project)?;
    }
    state
        .projects_storage
        .delete_project(id)
        .map_err(|e| e.into())
}

#[tauri::command]
fn list_deltas(
    state: State<'_, AppState>,
    project_id: &str,
) -> Result<HashMap<String, Vec<Delta>>, Error> {
    if let Some(project) = state.projects_storage.get_project(project_id)? {
        let project_path = Path::new(&project.path);
        let deltas = deltas::list_current_deltas(project_path)?;
        Ok(deltas)
    } else {
        Err("Project not found".into())
    }
}

fn main() {
    let colors = ColoredLevelConfig {
        error: Color::Red,
        warn: Color::Yellow,
        debug: Color::Blue,
        info: Color::BrightGreen,
        trace: Color::Cyan,
    };

    tauri::Builder::default()
        .setup(move |app| {
            let resolver = app.path_resolver();
            let storage = Storage::new(&resolver);
            let projects_storage = projects::Storage::new(storage);

            let watchers = watchers::WatcherCollection::default();

            if let Ok(projects) = projects_storage.list_projects() {
                for project in projects {
                    watchers::watch(app.get_window("main").unwrap(), &watchers, &project)
                        .map_err(|e| e.to_string())?;
                }
            } else {
                log::error!("Failed to list projects");
            }

            app.manage(AppState {
                watchers,
                projects_storage,
            });

            Ok(())
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .with_colors(colors)
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            read_project_file,
            list_project_files,
            add_project,
            list_projects,
            delete_project,
            list_deltas
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    message: String,
}

impl From<deltas::Error> for Error {
    fn from(error: deltas::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl From<projects::StorageError> for Error {
    fn from(error: projects::StorageError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}
