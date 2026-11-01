//! 配置文件读写：JSON 格式，保存笔记目录和悬浮球位置。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    /// 笔记保存目录
    pub save_dir: Option<String>,
    /// 悬浮球窗口位置（逻辑像素）
    pub ball_x: Option<f32>,
    pub ball_y: Option<f32>,
}

/// 配置文件路径：%APPDATA%\desk_tool\config.json（其他平台为对应的配置目录）
fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("desk_tool").join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(cfg: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(&path, text);
    }
}
