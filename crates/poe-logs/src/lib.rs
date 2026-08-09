use anyhow::{Context, Result};
use poe_core::{parse_client_line, GameEvent};
use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
    time::Duration,
};

pub fn common_log_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(program_files) = std::env::var("PROGRAMFILES(X86)") {
        paths.push(
            PathBuf::from(program_files).join("Grinding Gear Games/Path of Exile/logs/Client.txt"),
        );
    }
    if let Ok(steam) = std::env::var("STEAM_COMPAT_DATA_PATH") {
        paths.push(PathBuf::from(steam).join(
            "pfx/drive_c/Program Files (x86)/Grinding Gear Games/Path of Exile/logs/Client.txt",
        ));
    }
    paths
}

pub fn spawn_tail(
    path: PathBuf,
    sender: Sender<GameEvent>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<Result<()>> {
    thread::spawn(move || tail(&path, sender, stop))
}

fn tail(path: &Path, sender: Sender<GameEvent>, stop: Arc<AtomicBool>) -> Result<()> {
    let file = File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let file_len = reader.get_ref().metadata()?.len();
    let replay_start = file_len.saturating_sub(256 * 1024);
    reader.seek(SeekFrom::Start(replay_start))?;
    if replay_start > 0 {
        let mut partial_line = String::new();
        reader.read_line(&mut partial_line)?;
    }
    while !stop.load(Ordering::Relaxed) {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            thread::sleep(Duration::from_millis(250));
            continue;
        }
        if let Some(event) = parse_client_line(&line) {
            if sender.send(event).is_err() {
                break;
            }
        }
    }
    Ok(())
}
