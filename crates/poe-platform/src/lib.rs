use std::path::PathBuf;
use sysinfo::System;

pub fn is_poe_running() -> bool {
    let system = System::new_all();
    system.processes().values().any(|process| {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        is_poe_process_name(&name)
    })
}

fn is_poe_process_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized == "pathofexile"
        || normalized == "pathofexile.exe"
        || normalized.starts_with("pathofexile_")
        || normalized.starts_with("pathofexilesteam")
        // Linux /proc comm names may be truncated to 15 characters.
        || (normalized.starts_with("pathofexile") && normalized.len() >= 13)
}

pub fn discover_client_log() -> Option<PathBuf> {
    candidate_log_paths()
        .into_iter()
        .find(|path| path.is_file())
}

pub fn candidate_log_paths() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(program_files) = std::env::var("PROGRAMFILES(X86)") {
        let root = PathBuf::from(program_files);
        roots.push(root.join("Grinding Gear Games/Path of Exile/logs/Client.txt"));
        roots.push(root.join("Steam/steamapps/common/Path of Exile/logs/Client.txt"));
    }
    if let Ok(program_files) = std::env::var("PROGRAMFILES") {
        roots.push(
            PathBuf::from(program_files).join("Grinding Gear Games/Path of Exile/logs/Client.txt"),
        );
    }
    if let Ok(steam) = std::env::var("STEAM_COMPAT_DATA_PATH") {
        roots.push(PathBuf::from(steam).join(
            "pfx/drive_c/Program Files (x86)/Grinding Gear Games/Path of Exile/logs/Client.txt",
        ));
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        roots.push(
            PathBuf::from(user_profile).join("Documents/My Games/Path of Exile/logs/Client.txt"),
        );
    }
    if let Ok(user_home) = std::env::var("HOME") {
        let home = PathBuf::from(user_home);
        for steam_root in [
            home.join(".steam/steam"),
            home.join(".local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        ] {
            roots.push(steam_root.join("steamapps/common/Path of Exile/logs/Client.txt"));
            roots.push(steam_root.join(
                "steamapps/compatdata/238960/pfx/drive_c/users/steamuser/Documents/My Games/Path of Exile/logs/Client.txt",
            ));
        }
    }

    roots.sort();
    roots.dedup();
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_end_in_client_log() {
        assert!(candidate_log_paths()
            .iter()
            .all(|path| { path.file_name().is_some_and(|name| name == "Client.txt") }));
    }

    #[test]
    fn recognizes_native_windows_and_truncated_proton_names() {
        for name in [
            "PathOfExile.exe",
            "PathOfExile_x64.exe",
            "PathOfExileSteam.exe",
            "PathOfExileSte",
        ] {
            assert!(is_poe_process_name(name), "did not recognize {name}");
        }
        assert!(!is_poe_process_name("poe-app"));
    }
}
