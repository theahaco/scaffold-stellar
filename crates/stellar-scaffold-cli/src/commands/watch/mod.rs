use clap::Parser;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{self, RecursiveMode, Watcher as _};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use stellar_cli::print::Print;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time;

use crate::commands::build::{self, env_toml};

use super::build::clients::ScaffoldEnv;
use super::build::env_toml::ENV_FILE;

pub enum Message {
    FileChanged,
}

#[derive(Parser, Debug, Clone)]
#[group(skip)]
pub struct Cmd {
    #[command(flatten)]
    pub build_cmd: build::Command,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Watcher(#[from] notify::Error),
    #[error(transparent)]
    Build(#[from] build::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Env(#[from] env_toml::Error),
    #[error("Failed to start docker container")]
    DockerStart,
    #[error(transparent)]
    Manifest(#[from] cargo_metadata::Error),
}

fn canonicalize_path(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else if path.components().count() == 1 {
        // Path is a single component, assuming it's a filename
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    } else {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }
}

#[derive(Clone)]
pub struct Watcher {
    env_toml_dir: Arc<PathBuf>,
    packages: Arc<Vec<PathBuf>>,
    ignores: Arc<Gitignore>,
}

impl Watcher {
    pub fn new(env_toml_dir: &Path, packages: &[PathBuf]) -> Self {
        let env_toml_dir: Arc<PathBuf> = Arc::new(canonicalize_path(env_toml_dir));
        let packages: Arc<Vec<PathBuf>> =
            Arc::new(packages.iter().map(|p| canonicalize_path(p)).collect());

        let mut builder = GitignoreBuilder::new(&*env_toml_dir);
        for package in packages.iter() {
            builder.add(package);
        }

        let ignores = Arc::new(builder.build().expect("Failed to build GitIgnore"));

        Self {
            env_toml_dir,
            packages,
            ignores,
        }
    }

    pub fn is_watched(&self, path: &Path) -> bool {
        let path = canonicalize_path(path);
        !self.ignores.matched(&path, path.is_dir()).is_ignore()
    }

    pub fn is_env_toml(&self, path: &Path) -> bool {
        path == self.env_toml_dir.join(ENV_FILE)
    }

    pub fn handle_event(&self, event: &notify::Event, tx: &mpsc::Sender<Message>) {
        if matches!(
            event.kind,
            notify::EventKind::Create(notify::event::CreateKind::File)
                | notify::EventKind::Modify(notify::event::ModifyKind::Data(_))
                | notify::EventKind::Remove(notify::event::RemoveKind::File)
        ) {
            let watched_file = event.paths.iter().find(|path| {
                let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                    return false;
                };
                if ext.eq_ignore_ascii_case("toml") {
                    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                        return false;
                    };
                    if stem.eq_ignore_ascii_case("environments")
                        || stem.eq_ignore_ascii_case("cargo")
                    {
                        return self.is_watched(path);
                    }
                } else if ext.eq_ignore_ascii_case("rs") {
                    return self.is_watched(path);
                }
                false
            });

            if let Some(path) = watched_file {
                eprintln!("File changed: {}", path.display());
                if let Err(e) = tx.blocking_send(Message::FileChanged) {
                    eprintln!("Error sending through channel: {e:?}");
                }
            }
        }
    }
}

impl Cmd {
    pub async fn run(
        &mut self,
        global_args: &stellar_cli::commands::global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        let rebuild_state = Arc::new(Mutex::new(false));
        let metadata = &self.build_cmd.metadata()?;
        let env_toml_dir = metadata.workspace_root.as_std_path();
        if env_toml::Environment::get(env_toml_dir, &ScaffoldEnv::Development)?.is_none() {
            return Ok(());
        }
        let packages = self
            .build_cmd
            .list_packages(metadata)?
            .into_iter()
            .map(|package| {
                package
                    .manifest_path
                    .parent()
                    .unwrap()
                    .to_path_buf()
                    .into_std_path_buf()
            })
            .collect::<Vec<_>>();

        let watcher = Watcher::new(env_toml_dir, &packages);

        for package_path in watcher.packages.iter() {
            printer.infoln(format!("Watching {}", package_path.display()));
        }

        let mut notify_watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    watcher.handle_event(&event, &tx);
                }
            })
            .unwrap();

        notify_watcher.watch(
            &canonicalize_path(env_toml_dir),
            RecursiveMode::NonRecursive,
        )?;
        for package_path in packages {
            notify_watcher.watch(&canonicalize_path(&package_path), RecursiveMode::Recursive)?;
        }

        let build_command = self.cloned_build_command(global_args);
        if let Err(e) = build_command.0.run(&build_command.1).await {
            printer.errorln(format!("Build error: {e}"));
        }
        printer.infoln("Watching for changes. Press Ctrl+C to stop.");

        let rebuild_state_clone = rebuild_state.clone();
        let printer_clone = printer.clone();
        loop {
            tokio::select! {
                _ = rx.recv() => {
                    let mut state = rebuild_state_clone.lock().await;
                    let build_command_inner = build_command.clone();
                    if !*state {
                        *state = true;
                        tokio::spawn(Self::debounced_rebuild(build_command_inner, Arc::clone(&rebuild_state_clone), printer_clone.clone()));
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    printer.infoln("Stopping dev mode.");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn debounced_rebuild(
        build_command: Arc<(build::Command, stellar_cli::commands::global::Args)>,
        rebuild_state: Arc<Mutex<bool>>,
        printer: Print,
    ) {
        // Debounce to avoid multiple rapid rebuilds
        time::sleep(std::time::Duration::from_secs(1)).await;

        printer.infoln("Changes detected. Rebuilding...");
        if let Err(e) = build_command.0.run(&build_command.1).await {
            printer.errorln(format!("Build error: {e}"));
        }
        printer.infoln("Watching for changes. Press Ctrl+C to stop.");

        let mut state = rebuild_state.lock().await;
        *state = false;
    }

    fn cloned_build_command(
        &mut self,
        global_args: &stellar_cli::commands::global::Args,
    ) -> Arc<(build::Command, stellar_cli::commands::global::Args)> {
        self.build_cmd
            .build_clients_args
            .env
            .get_or_insert(ScaffoldEnv::Development);
        Arc::new((self.build_cmd.clone(), global_args.clone()))
    }
}
