use std::env;
use std::path::PathBuf;

pub struct TermuxInfo {
    pub is_termux: bool,
    pub home_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub storage_dir: PathBuf,
}

impl TermuxInfo {
    pub fn new() -> Self {
        let is_termux = Self::detect_termux();
        let home_dir = Self::get_home_dir();
        let data_dir = Self::get_data_dir();
        let cache_dir = Self::get_cache_dir();
        let config_dir = Self::get_config_dir();
        let storage_dir = Self::get_storage_dir();

        Self {
            is_termux,
            home_dir,
            data_dir,
            cache_dir,
            config_dir,
            storage_dir,
        }
    }

    pub fn detect_termux() -> bool {
        if env::var("TERMUX_VERSION").is_ok() {
            return true;
        }
        if env::var("PREFIX").is_ok() {
            let prefix = env::var("PREFIX").unwrap_or_default();
            if prefix.contains("/data/data/com.termux") {
                return true;
            }
        }
        if PathBuf::from("/data/data/com.termux").exists() {
            return true;
        }
        false
    }

    pub fn get_home_dir() -> PathBuf {
        if Self::detect_termux() {
            if let Ok(home) = env::var("HOME") {
                PathBuf::from(home)
            } else {
                PathBuf::from("/data/data/com.termux/files/home")
            }
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
        }
    }

    pub fn get_data_dir() -> PathBuf {
        if Self::detect_termux() {
            if let Ok(prefix) = env::var("PREFIX") {
                PathBuf::from(prefix).join("share")
            } else {
                PathBuf::from("/data/data/com.termux/files/usr/share")
            }
        } else {
            dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")
        }
    }

    pub fn get_cache_dir() -> PathBuf {
        if Self::detect_termux() {
            PathBuf::from("/data/data/com.termux/cache")
        } else {
            dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")
        }
    }

    pub fn get_config_dir() -> PathBuf {
        if Self::detect_termux() {
            Self::get_home_dir().join(".termux")
        } else {
            dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")
        }
    }

    pub fn get_storage_dir() -> PathBuf {
        if Self::detect_termux() {
            PathBuf::from("/storage/emulated/0")
        } else {
            Self::get_home_dir()
        }
    }

    pub fn get_external_storage_dir() -> Option<PathBuf> {
        if !Self::detect_termux() {
            return None;
        }
        let storage = PathBuf::from("/storage/emulated/0");
        if storage.exists() {
            Some(storage)
        } else {
            None
        }
    }

    pub fn get_downloads_dir() -> PathBuf {
        if Self::detect_termux() {
            Self::get_storage_dir().join("Download")
        } else {
            dirs::download_dir().unwrap_or_else(|| Self::get_home_dir())
        }
    }

    pub fn get_documents_dir() -> PathBuf {
        if Self::detect_termux() {
            Self::get_storage_dir().join("Documents")
        } else {
            dirs::document_dir().unwrap_or_else(|| Self::get_home_dir())
        }
    }

    pub fn get_pictures_dir() -> PathBuf {
        if Self::detect_termux() {
            Self::get_storage_dir().join("Pictures")
        } else {
            dirs::picture_dir().unwrap_or_else(|| Self::get_home_dir())
        }
    }

    pub fn get_music_dir() -> PathBuf {
        if Self::detect_termux() {
            Self::get_storage_dir().join("Music")
        } else {
            dirs::audio_dir().unwrap_or_else(|| Self::get_home_dir())
        }
    }

    pub fn get_movies_dir() -> PathBuf {
        if Self::detect_termux() {
            Self::get_storage_dir().join("Movies")
        } else {
            dirs::video_dir().unwrap_or_else(|| Self::get_home_dir())
        }
    }

    pub fn expand_path(path: &str) -> PathBuf {
        let path_str = path.to_string();
        
        if path_str.starts_with('~') {
            let home = Self::get_home_dir();
            PathBuf::from(path_str.replace('~', home.to_str().unwrap_or("")))
        } else if path_str.starts_with("/storage/") || path_str.starts_with("/sdcard/") {
            PathBuf::from(path_str)
        } else if path_str.starts_with("./") || path_str.starts_with(".\\") {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(&path_str[2..])
        } else if !path_str.starts_with('/') {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path_str)
        } else {
            PathBuf::from(path_str)
        }
    }

    pub fn is_path_accessible(path: &PathBuf) -> bool {
        match std::fs::metadata(path) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn request_storage_access() -> bool {
        if !Self::detect_termux() {
            return true;
        }
        
        let storage = Self::get_storage_dir();
        if storage.exists() && Self::is_path_accessible(&storage) {
            return true;
        }

        match std::process::Command::new("termux-setup-storage")
            .spawn()
        {
            Ok(mut child) => match child.wait() {
                Ok(status) => status.success(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    pub fn get_termux_version() -> Option<String> {
        if !Self::detect_termux() {
            return None;
        }
        env::var("TERMUX_VERSION").ok()
    }

    pub fn get_termux_arch() -> Option<String> {
        if !Self::detect_termux() {
            return None;
        }
        env::var("TERMUX_ARCH").ok()
    }

    pub fn get_termux_api_version() -> Option<String> {
        if !Self::detect_termux() {
            return None;
        }
        match std::process::Command::new("termux-api-version").output() {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    Some(version)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn toast(message: &str) {
        if !Self::detect_termux() {
            return;
        }
        let _ = std::process::Command::new("termux-toast")
            .arg(message)
            .spawn();
    }

    pub fn vibrate(duration_ms: u32) {
        if !Self::detect_termux() {
            return;
        }
        let _ = std::process::Command::new("termux-vibrate")
            .arg("--duration")
            .arg(duration_ms.to_string())
            .spawn();
    }

    pub fn clipboard_get() -> Option<String> {
        if !Self::detect_termux() {
            return None;
        }
        match std::process::Command::new("termux-clipboard-get").output() {
            Ok(output) => {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn clipboard_set(text: &str) {
        if !Self::detect_termux() {
            return;
        }
        let mut child = match std::process::Command::new("termux-clipboard-set")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        
        if let Some(mut stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }

    pub fn battery_status() -> Option<String> {
        if !Self::detect_termux() {
            return None;
        }
        match std::process::Command::new("termux-battery-status").output() {
            Ok(output) => {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn brightness_set(percent: u8) {
        if !Self::detect_termux() {
            return;
        }
        let _ = std::process::Command::new("termux-brightness")
            .arg(percent.to_string())
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_termux() {
        let info = TermuxInfo::new();
        println!("Is Termux: {}", info.is_termux);
        println!("Home: {:?}", info.home_dir);
    }

    #[test]
    fn test_expand_path() {
        let path = TermuxInfo::expand_path("~/test");
        println!("Expanded path: {:?}", path);
    }

    #[test]
    fn test_storage_dir() {
        let info = TermuxInfo::new();
        println!("Storage dir: {:?}", info.storage_dir);
    }
}