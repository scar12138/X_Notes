//! 导出 / 导入全部数据（JSON + 图片打包成一个 zip）

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub fn export_all(data_dir: &Path, images_dir: &Path, dest: &Path) -> Result<usize, String> {
    let file = File::create(dest).map_err(|e| format!("无法创建导出文件：{e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0usize;

    for name in ["notes.json", "settings.json"] {
        let path = data_dir.join(name);
        if path.exists() {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    if images_dir.exists() {
        let entries = fs::read_dir(images_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            zip.start_file(format!("images/{name}"), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(count)
}

fn safe_entry(name: &str) -> Option<String> {
    if name.contains("..") || name.starts_with('/') || name.contains('\\') {
        return None;
    }
    let ok = name == "notes.json"
        || name == "settings.json"
        || (name.starts_with("images/") && name.len() > "images/".len());
    if ok {
        Some(name.to_string())
    } else {
        None
    }
}

pub fn import_all(data_dir: &Path, images_dir: &Path, src: &Path) -> Result<usize, String> {
    let file = File::open(src).map_err(|e| format!("无法打开导入文件：{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("不是有效的便签数据包：{e}"))?;
    fs::create_dir_all(images_dir).map_err(|e| e.to_string())?;

    // 先把 notes.json 校验一遍，避免导入坏包把现有数据覆盖
    let mut notes_ok = false;
    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            if entry.name() == "notes.json" {
                let mut text = String::new();
                entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
                let value: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("数据包里的 notes.json 无效：{e}"))?;
                if !value.is_array() && value.get("notes").is_none() {
                    return Err("数据包里的 notes.json 缺少 notes 字段".into());
                }
                notes_ok = true;
            }
        }
    }
    if !notes_ok {
        return Err("数据包里没有 notes.json，无法导入".into());
    }

    // 覆盖前先备份当前数据
    let notes_path = data_dir.join("notes.json");
    if notes_path.exists() {
        let mut backup = std::ffi::OsString::from(notes_path.as_os_str());
        backup.push(".pre-import");
        let _ = fs::copy(&notes_path, Path::new(&backup));
    }

    let file = File::open(src).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = match safe_entry(entry.name()) {
            Some(n) => n,
            None => continue,
        };
        let dest = data_dir.join(&name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        fs::write(&dest, &bytes).map_err(|e| format!("写入 {name} 失败：{e}"))?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_and_import_roundtrip() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "x-notes-export-test-{}",
                uuid::Uuid::new_v4().simple()
            ));
        let src_data = base.join("src");
        let src_images = src_data.join("images");
        fs::create_dir_all(&src_images).unwrap();
        fs::write(src_data.join("notes.json"), r#"{"dataVersion":1,"notes":[]}"#).unwrap();
        fs::write(src_data.join("settings.json"), r#"{"autostart":false}"#).unwrap();
        fs::write(src_images.join("img-test.png"), b"fake-png-bytes").unwrap();

        let zip_path = base.join("backup.zip");
        let exported = export_all(&src_data, &src_images, &zip_path).unwrap();
        assert_eq!(exported, 3);

        let dst_data = base.join("dst");
        let dst_images = dst_data.join("images");
        let imported = import_all(&dst_data, &dst_images, &zip_path).unwrap();
        assert_eq!(imported, 3);
        assert_eq!(
            fs::read_to_string(dst_data.join("notes.json")).unwrap(),
            r#"{"dataVersion":1,"notes":[]}"#
        );
        assert_eq!(
            fs::read(dst_images.join("img-test.png")).unwrap(),
            b"fake-png-bytes"
        );

        let _ = fs::remove_dir_all(&base);
    }
}
