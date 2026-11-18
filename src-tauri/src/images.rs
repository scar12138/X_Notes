use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use base64::Engine;
use image::ImageReader;

fn allowed_ext(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "png" => Some("png"),
        "jpg" => Some("jpg"),
        "jpeg" => Some("jpeg"),
        "webp" => Some("webp"),
        "gif" => Some("gif"),
        _ => None,
    }
}

fn uniq_name(ext: &str) -> String {
    let raw = uuid::Uuid::new_v4().simple().to_string();
    format!("img-{}.{}", &raw[..12], ext)
}

/// 读取图片尺寸（不解码整张图，GIF/WebP 也适用）
pub fn image_dimensions(path: &Path) -> Result<(u32, u32), String> {
    let reader = ImageReader::open(path)
        .map_err(|e| format!("无法打开图片：{e}"))?
        .with_guessed_format()
        .map_err(|e| format!("无法识别图片格式：{e}"))?;
    reader
        .into_dimensions()
        .map_err(|e| format!("无法读取图片尺寸：{e}"))
}

/// 把用户选择的图片复制进数据目录的 images/，返回 (文件名, 宽, 高)
pub fn import_image_file(images_dir: &Path, source: &Path) -> Result<(String, u32, u32), String> {
    let ext = allowed_ext(source).ok_or_else(|| "只支持 PNG、JPG、JPEG、WebP、GIF 图片".to_string())?;
    let (w, h) = image_dimensions(source)?;
    if w == 0 || h == 0 {
        return Err("图片尺寸无效".into());
    }
    fs::create_dir_all(images_dir).map_err(|e| e.to_string())?;
    let name = uniq_name(ext);
    let dest = images_dir.join(&name);
    fs::copy(source, &dest).map_err(|e| format!("复制图片失败：{e}"))?;
    Ok((name, w, h))
}

/// 从剪贴板读取图片并保存为 PNG
pub fn import_clipboard_image(images_dir: &Path) -> Result<(String, u32, u32), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板：{e}"))?;
    let data = clipboard
        .get_image()
        .map_err(|e| format!("剪贴板里没有图片：{e}"))?;
    let (w, h) = (data.width as u32, data.height as u32);
    if w == 0 || h == 0 {
        return Err("剪贴板图片尺寸无效".into());
    }
    let img = image::RgbaImage::from_raw(w, h, data.bytes.to_vec())
        .ok_or_else(|| "剪贴板图片数据损坏".to_string())?;
    fs::create_dir_all(images_dir).map_err(|e| e.to_string())?;
    let name = uniq_name("png");
    let dest = images_dir.join(&name);
    img.save(&dest).map_err(|e| format!("保存图片失败：{e}"))?;
    Ok((name, w, h))
}

fn mime_of(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

/// 读取图片并以 data URL 返回给前端（避免配置 asset 协议作用域）
pub fn read_data_url(images_dir: &Path, name: &str) -> Result<String, String> {
    let path = safe_image_path(images_dir, name)?;
    let bytes = fs::read(&path).map_err(|e| format!("读取图片失败：{e}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", mime_of(name), encoded))
}

/// 列表缩略图缓存：文件名 -> (修改时间, data URL)。
/// 图片名由 uuid 生成、正常不会被改写，记录修改时间只是为了手工替换文件时也能刷新。
fn thumb_cache() -> &'static Mutex<HashMap<String, (Option<SystemTime>, String)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Option<SystemTime>, String)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 缩略图最长边上限：管理器列表的 .thumb 只有 48px，2 倍屏加余量 144px 够用
const THUMB_MAX: u32 = 144;

/// 生成列表用缩略图（最长边 144px 的 PNG data URL）。
/// 列表刷新很频繁，整张原图 base64 到前端代价太大；这里在 Rust 侧缩放并缓存结果。
pub fn read_thumbnail_data_url(images_dir: &Path, name: &str) -> Result<String, String> {
    let path = safe_image_path(images_dir, name)?;
    let stamp = fs::metadata(&path).and_then(|m| m.modified()).ok();

    if let Ok(cache) = thumb_cache().lock() {
        if let Some((cached, url)) = cache.get(name) {
            if *cached == stamp {
                return Ok(url.clone());
            }
        }
    }

    let img = image::open(&path).map_err(|e| format!("无法解码图片：{e}"))?;
    let thumb = img.thumbnail(THUMB_MAX, THUMB_MAX);
    let mut buf = Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("生成缩略图失败：{e}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    let url = format!("data:image/png;base64,{encoded}");

    if let Ok(mut cache) = thumb_cache().lock() {
        // 手工删改数据目录时缓存可能残留，简单设个上限，避免无限增长
        if cache.len() > 512 {
            cache.clear();
        }
        cache.insert(name.to_string(), (stamp, url.clone()));
    }
    Ok(url)
}

/// 防止通过文件名跳出 images 目录
pub fn safe_image_path(images_dir: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
    {
        return Err("非法的图片文件名".into());
    }
    let path = images_dir.join(name);
    if !path.exists() {
        return Err(format!("图片文件不存在：{name}"));
    }
    Ok(path)
}

pub fn export_image_copy(images_dir: &Path, name: &str, dest: &Path) -> Result<(), String> {
    let src = safe_image_path(images_dir, name)?;
    fs::copy(&src, dest).map_err(|e| format!("导出图片失败：{e}"))?;
    Ok(())
}

/// 图片显示尺寸：按最长边不超过 max 等比例缩小
pub fn fit_size(w: u32, h: u32, max: f64) -> (f64, f64) {
    let fw = w as f64;
    let fh = h as f64;
    let scale = (max / fw.max(fh)).min(1.0);
    let mut dw = (fw * scale).round();
    let mut dh = (fh * scale).round();
    // 最小显示尺寸也要按比例放大：分别兜底 60/40 会把宽高比改掉
    if dw < 60.0 || dh < 40.0 {
        let grow = (60.0 / dw.max(1.0)).max(40.0 / dh.max(1.0));
        dw = (dw * grow).round();
        dh = (dh * grow).round();
    }
    (dw.max(60.0), dh.max(40.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_shrinks_and_is_cached() {
        let dir = std::env::temp_dir().join(format!("sn-thumb-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let name = "t.png";
        let img = image::RgbaImage::from_pixel(600, 300, image::Rgba([200, 100, 50, 255]));
        img.save(dir.join(name)).unwrap();

        let thumb = read_thumbnail_data_url(&dir, name).unwrap();
        assert!(thumb.starts_with("data:image/png;base64,"));
        // 600x300 缩到最长边 144，base64 体积必须明显小于原图
        let full = read_data_url(&dir, name).unwrap();
        assert!(thumb.len() < full.len());
        // 第二次走缓存，结果完全一致
        assert_eq!(thumb, read_thumbnail_data_url(&dir, name).unwrap());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn thumbnail_rejects_path_escape() {
        let dir = std::env::temp_dir();
        assert!(read_thumbnail_data_url(&dir, "../notes.json").is_err());
    }
}
