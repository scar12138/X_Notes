use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::models::{Category, Note, Settings};

/// 本地数据存储：notes.json + settings.json + images/
/// 写入策略：先写临时文件，成功后把旧文件复制成 .bak，再替换正式文件。
pub struct Storage {
    pub dir: PathBuf,
    pub images_dir: PathBuf,
    pub notes: Vec<Note>,
    pub categories: Vec<Category>,
    pub settings: Settings,
    /// 数据文件损坏、从备份恢复等需要提示用户的信息
    pub warning: Option<String>,
}

fn bak_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".bak");
    PathBuf::from(name)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".tmp");
    PathBuf::from(name)
}

pub fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path(path);
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    if path.exists() {
        // 保留最近一份备份
        let _ = fs::copy(path, bak_path(path));
    }
    match fs::rename(&tmp, path) {
        Ok(_) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

impl Storage {
    pub fn load(app: &AppHandle) -> Self {
        let dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let images_dir = dir.join("images");
        let _ = fs::create_dir_all(&images_dir);

        let notes_path = dir.join("notes.json");
        let settings_path = dir.join("settings.json");

        let mut warnings: Vec<String> = Vec::new();

        let mut notes: Vec<Note> = Vec::new();
        let mut categories: Vec<Category> = Vec::new();
        match fs::read_to_string(&notes_path) {
            Ok(text) => match parse_notes(&text) {
                Ok((list, cats)) => {
                    notes = list;
                    categories = cats;
                }
                Err(err) => {
                    // 先把损坏文件留证，再尝试 .bak。否则启动后的自动保存会把好备份覆盖掉。
                    let corrupt = dir.join("notes.json.corrupt");
                    let _ = fs::copy(&notes_path, &corrupt);
                    let bak = bak_path(&notes_path);
                    match fs::read_to_string(&bak).and_then(|t| {
                        parse_notes(&t)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    }) {
                        Ok((list, cats)) => {
                            notes = list;
                            categories = cats;
                            // 立刻用备份覆盖坏文件，让后续 .bak 保存链保持健康
                            let _ = fs::copy(&bak, &notes_path);
                            warnings.push(format!(
                                "便签数据文件损坏（{err}），已自动从 notes.json.bak 恢复；损坏文件保留为 notes.json.corrupt。"
                            ));
                        }
                        Err(_) => {
                            warnings.push(format!(
                                "便签数据文件损坏（{err}），且备份不可用；损坏文件保留为 notes.json.corrupt。"
                            ));
                        }
                    }
                }
            },
            Err(_) => {}
        }

        let mut settings = Settings::default();
        match fs::read_to_string(&settings_path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(parsed) => settings = parsed,
                Err(err) => warnings.push(format!(
                    "设置文件 settings.json 损坏（{err}），已使用默认设置。"
                )),
            },
            Err(_) => {}
        }
        settings.sanitize();

        for note in &mut notes {
            note.sanitize();
        }

        // 图片文件丢失的便签仍然保留，但要明确告诉用户
        let missing = notes
            .iter()
            .filter(|note| {
                note.image_file
                    .as_ref()
                    .is_some_and(|name| !images_dir.join(name).is_file())
            })
            .count();
        if missing > 0 {
            warnings.push(format!(
                "有 {missing} 张图片便签的图片文件丢失，便签仍保留，可重新导入图片。"
            ));
        }

        Self {
            dir,
            images_dir,
            notes,
            categories,
            settings,
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join("；"))
            },
        }
    }

    pub fn notes_path(&self) -> PathBuf {
        self.dir.join("notes.json")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    pub fn save_notes(&mut self) -> Result<(), String> {
        let payload = json!({
            "dataVersion": 2,
            "notes": self.notes,
            "categories": self.categories,
        });
        let text = serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?;
        atomic_write(&self.notes_path(), &text).map_err(|e| e.to_string())
    }

    pub fn save_settings(&mut self) -> Result<(), String> {
        self.settings.sanitize();
        let text = serde_json::to_vec_pretty(&self.settings).map_err(|e| e.to_string())?;
        atomic_write(&self.settings_path(), &text).map_err(|e| e.to_string())
    }

    pub fn get(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Note> {
        let pos = self.notes.iter().position(|n| n.id == id)?;
        Some(self.notes.remove(pos))
    }

    /// 分类是否存在（空 id 代表“未分类”，始终有效）
    pub fn category_exists(&self, id: &str) -> bool {
        id.is_empty() || self.categories.iter().any(|c| c.id == id)
    }
}

fn parse_notes(text: &str) -> Result<(Vec<Note>, Vec<Category>), String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    // 兼容两种格式：{ dataVersion, notes: [...], categories: [...] } 或直接是数组
    if value.is_array() {
        let notes = serde_json::from_value::<Vec<Note>>(value).map_err(|e| e.to_string())?;
        return Ok((notes, Vec::new()));
    }
    let raw = value
        .get("notes")
        .cloned()
        .ok_or_else(|| "缺少 notes 字段".to_string())?;
    let notes = serde_json::from_value::<Vec<Note>>(raw).map_err(|e| e.to_string())?;
    let categories = value
        .get("categories")
        .and_then(|v| serde_json::from_value::<Vec<Category>>(v.clone()).ok())
        .unwrap_or_default();
    Ok((notes, categories))
}
