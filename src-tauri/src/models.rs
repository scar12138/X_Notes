use serde::{Deserialize, Serialize};

/// 便签类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Text,
    Image,
}

impl Default for NoteType {
    fn default() -> Self {
        NoteType::Text
    }
}

/// 窗口层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// 置顶：始终在所有普通窗口前面
    Top,
    /// 桌面：只在桌面层显示，被其他窗口盖住
    Desktop,
    /// 普通：参与普通窗口层叠
    Normal,
}

impl Default for Layer {
    fn default() -> Self {
        Layer::Normal
    }
}

impl Layer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Layer::Top => "top",
            Layer::Desktop => "desktop",
            Layer::Normal => "normal",
        }
    }
}

/// 图片相框样式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameStyle {
    None,
    Thin,
    Rounded,
    Polaroid,
    Gallery,
}

impl Default for FrameStyle {
    fn default() -> Self {
        FrameStyle::None
    }
}

impl FrameStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameStyle::None => "none",
            FrameStyle::Thin => "thin",
            FrameStyle::Rounded => "rounded",
            FrameStyle::Polaroid => "polaroid",
            FrameStyle::Gallery => "gallery",
        }
    }
}

/// 阴影设置（每张便签独立）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Shadow {
    pub enabled: bool,
    pub color: String,
    /// 0.0 - 1.0
    pub opacity: f64,
    /// 0 - 40 px
    pub blur: f64,
    /// -30 - 30 px
    pub offset_x: f64,
    pub offset_y: f64,
    /// none / soft / hard / deep / neon / custom
    pub preset: String,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#000000".into(),
            opacity: 0.18,
            blur: 24.0,
            offset_x: 0.0,
            offset_y: 8.0,
            preset: "soft".into(),
        }
    }
}

/// 文字便签边线（默认关闭）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Border {
    pub enabled: bool,
    pub color: String,
    /// 1 - 8 px
    pub width: f64,
    pub corner_radius: f64,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            enabled: false,
            color: "#FFFFFF".into(),
            width: 2.0,
            corner_radius: 10.0,
        }
    }
}

/// 图片相框（和阴影、边线完全分开存）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Frame {
    pub style: FrameStyle,
    pub color: String,
    /// 相框宽度 / 边框粗细 1 - 30 px
    pub width: f64,
    /// 圆角半径 0 - 30 px
    pub corner_radius: f64,
    /// 拍立得样式专用的下边宽度
    pub frame_bottom_width: f64,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            style: FrameStyle::None,
            color: "#FFFFFF".into(),
            width: 3.0,
            corner_radius: 4.0,
            frame_bottom_width: 44.0,
        }
    }
}

/// 相框向内容外侧扩展的尺寸（左、上、右、下）
#[derive(Debug, Clone, Copy, Default)]
pub struct Insets {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Frame {
    /// 相框外扩尺寸；相框宽度算在窗口尺寸内，但图片显示尺寸不变（外扩）
    pub fn insets(&self) -> Insets {
        let w = self.width.clamp(0.0, 40.0);
        match self.style {
            FrameStyle::None => Insets::default(),
            FrameStyle::Polaroid => Insets {
                left: w,
                top: w,
                right: w,
                bottom: self.frame_bottom_width.clamp(w, 200.0),
            },
            _ => Insets {
                left: w,
                top: w,
                right: w,
                bottom: w,
            },
        }
    }
}

/// 便签分类（“未分类”是空 id，不占用分类列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Category {
    pub id: String,
    pub name: String,
}

impl Default for Category {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "未分类".into(),
        }
    }
}

/// 按 `ids` 给出的顺序重排分类；没出现在 `ids` 里的分类按原顺序补在末尾，
/// 保证排序动作永远不会丢分类。
pub fn order_categories(current: &[Category], ids: &[String]) -> Vec<Category> {
    let mut ordered: Vec<Category> = Vec::with_capacity(current.len());
    for id in ids {
        if let Some(cat) = current.iter().find(|c| &c.id == id) {
            if !ordered.iter().any(|c| &c.id == id) {
                ordered.push(cat.clone());
            }
        }
    }
    for cat in current {
        if !ordered.iter().any(|c| c.id == cat.id) {
            ordered.push(cat.clone());
        }
    }
    ordered
}

/// 一张便签的全部数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Note {
    pub id: String,
    #[serde(rename = "type")]
    pub note_type: NoteType,
    pub content: String,
    pub image_file: Option<String>,

    /// 窗口左上角坐标（逻辑像素）
    pub x: f64,
    pub y: f64,
    /// 内容区尺寸（不含相框外扩与阴影留白）
    pub width: f64,
    pub height: f64,

    pub layer: Layer,

    pub background_color: String,
    pub text_color: String,
    pub font_family: String,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub text_align: String,

    /// 0.1 - 1.0
    pub opacity: f64,
    pub corner_radius: f64,

    pub shadow: Shadow,
    pub border: Border,
    pub frame: Frame,

    pub locked: bool,
    pub visible: bool,
    pub click_through: bool,
    /// 图片便签拖动改大小时是否锁定比例（默认开）
    pub lock_aspect: bool,

    /// 分类 id；空字符串表示“未分类”
    pub category: String,
    /// 放进回收站的时间；None 表示正常便签
    pub deleted_at: Option<String>,

    pub created_at: String,
    pub updated_at: String,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            id: String::new(),
            note_type: NoteType::Text,
            content: String::new(),
            image_file: None,
            x: 120.0,
            y: 120.0,
            width: 280.0,
            height: 200.0,
            layer: Layer::Normal,
            background_color: "#FFF1A8".into(),
            text_color: "#222222".into(),
            font_family: "Microsoft YaHei".into(),
            font_size: 15.0,
            bold: false,
            italic: false,
            text_align: "left".into(),
            opacity: 1.0,
            corner_radius: 10.0,
            shadow: Shadow::default(),
            border: Border::default(),
            frame: Frame::default(),
            locked: false,
            visible: true,
            click_through: false,
            lock_aspect: true,
            category: String::new(),
            deleted_at: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

impl Note {
    /// 约束所有可调参数的合法范围，避免前端传入越界值
    pub fn sanitize(&mut self) {
        if self.shadow.preset == "soft"
            && self.shadow.blur == 16.0
            && self.shadow.offset_x == 0.0
            && self.shadow.offset_y == 4.0
            && (self.shadow.opacity - 0.25).abs() < 1e-9
        {
            // 旧版柔和参数（0 4px 16px）升级为 0 8px 24px
            self.shadow = shadow_preset("soft");
        }
        // 尺寸：文字便签宽高各自夹；图片便签锁定比例时宽高必须一起夹，
        // 分别夹会把宽高比夹坏（缩到最小时被压扁就是这么来的）
        if self.note_type == NoteType::Image
            && self.lock_aspect
            && self.width > 0.0
            && self.height > 0.0
        {
            let ratio = self.width / self.height;
            // 最小/最大尺寸按原图比例折算：宽到最小就停，高跟着停
            let min_w = 80.0f64.max(60.0 * ratio);
            let max_w = 3000.0f64.min(3000.0 * ratio);
            if min_w <= max_w {
                if self.width < min_w || self.width > max_w {
                    self.width = self.width.clamp(min_w, max_w);
                    self.height = self.width / ratio;
                } else if self.height < 60.0 || self.height > 3000.0 {
                    self.height = self.height.clamp(60.0, 3000.0);
                    self.width = self.height * ratio;
                }
            } else {
                self.width = self.width.clamp(80.0, 3000.0);
                self.height = self.height.clamp(60.0, 3000.0);
            }
        } else {
            self.width = self.width.clamp(80.0, 3000.0);
            self.height = self.height.clamp(60.0, 3000.0);
        }
        self.font_size = self.font_size.clamp(9.0, 72.0);
        self.opacity = self.opacity.clamp(0.1, 1.0);
        self.corner_radius = self.corner_radius.clamp(0.0, 40.0);
        self.shadow.blur = self.shadow.blur.clamp(0.0, 40.0);
        self.shadow.opacity = self.shadow.opacity.clamp(0.0, 1.0);
        self.shadow.offset_x = self.shadow.offset_x.clamp(-30.0, 30.0);
        self.shadow.offset_y = self.shadow.offset_y.clamp(-30.0, 30.0);
        self.shadow.color = normalize_color(&self.shadow.color);
        self.border.width = self.border.width.clamp(1.0, 8.0);
        self.border.corner_radius = self.border.corner_radius.clamp(0.0, 30.0);
        self.border.color = normalize_color(&self.border.color);
        self.frame.width = self.frame.width.clamp(1.0, 30.0);
        self.frame.corner_radius = self.frame.corner_radius.clamp(0.0, 30.0);
        self.frame.frame_bottom_width = self.frame.frame_bottom_width.clamp(8.0, 200.0);
        self.frame.color = normalize_color(&self.frame.color);
        // 旧版高饱和背景按色相映射到低饱和淡色；已经是淡色的保持不变
        self.background_color = pastel_background(&self.background_color);
        self.text_color = normalize_color(&self.text_color);
        if self.text_align != "center" {
            self.text_align = "left".into();
        }
        // 拍立得样式下，下边宽度不能小于左右宽度
        if self.frame.style == FrameStyle::Polaroid {
            self.frame.frame_bottom_width = self
                .frame
                .frame_bottom_width
                .max(self.frame.width.clamp(1.0, 30.0));
        }
    }

    /// 是否在回收站里
    pub fn is_trashed(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// 相框外扩（文字便签恒为 0）
    pub fn frame_insets(&self) -> Insets {        if self.note_type == NoteType::Image {
            self.frame.insets()
        } else {
            Insets::default()
        }
    }

    /// 内容区到窗口边缘的总留白：只有相框，阴影不占窗口空间
    pub fn window_insets(&self) -> Insets {
        self.frame_insets()
    }

    /// 窗口逻辑尺寸 = 内容尺寸 + 相框外扩（窗口与可见便签等大）
    pub fn window_size(&self) -> (f64, f64) {
        let i = self.window_insets();
        (self.width + i.left + i.right, self.height + i.top + i.bottom)
    }

    /// 相框 / 阴影参数改变后平移窗口左上角，让内容区在屏幕上保持原位。
    ///
    /// 传入修改前的总留白。因为内容 left = 窗口 x + 左留白，
    /// 所以新 x = 旧 x + 旧左留白 - 新左留白。上下同理。
    pub fn anchor_content_after_insets_change(&mut self, old_insets: Insets) {
        let new_insets = self.window_insets();
        self.x += old_insets.left - new_insets.left;
        self.y += old_insets.top - new_insets.top;
    }
}

/// 便签背景用的淡色板：明度 82~90%、饱和度 45~100%（视觉上约 50% 上下），
/// 黄 / 绿 / 蓝 / 粉 / 紫 / 橙 六个色相一眼能分开，长时间看不刺眼
pub const PASTEL_BACKGROUNDS: [&str; 6] = [
    "#FFF1A8", // 淡黄
    "#B7E4C7", // 淡绿
    "#A8D8FF", // 淡蓝
    "#FFC8DD", // 淡粉
    "#D8B4FE", // 淡紫
    "#FFD6A5", // 淡橙
];

/// #RRGGBB -> (色相, 饱和度%, 明度%)
fn rgb_to_hsl(hex: &str) -> (f64, f64, f64) {
    let (r, g, b) = hex_to_rgb(hex);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f64::EPSILON {
        return (0.0, 0.0, l * 100.0);
    }
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < f64::EPSILON {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if (max - g).abs() < f64::EPSILON {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };
    (h, s * 100.0, l * 100.0)
}

/// #RRGGBB -> 三个 0~1 的分量
fn hex_to_rgb(hex: &str) -> (f64, f64, f64) {
    let parse = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f64 / 255.0;
    (parse(1), parse(3), parse(5))
}

/// 两个颜色的 RGB 欧氏距离平方（只在色相打平时用来挑更接近的那个）
fn rgb_distance(a: &str, b: &str) -> f64 {
    let (ar, ag, ab) = hex_to_rgb(a);
    let (br, bg, bb) = hex_to_rgb(b);
    let dr = ar - br;
    let dg = ag - bg;
    let db = ab - bb;
    dr * dr + dg * dg + db * db
}

/// 便签背景统一成低饱和淡色。
///
/// 色板里的颜色原样保留；黑白灰这类无彩色也原样保留（不去动用户的深浅选择）。
/// 旧版的高饱和色、以及早先那套偏灰的淡色，按色相映射到最接近的淡色，
/// 不会直接变成白色。
pub fn pastel_background(value: &str) -> String {
    let hex = normalize_color(value);
    if PASTEL_BACKGROUNDS.iter().any(|c| *c == hex) {
        return hex;
    }
    let (h, s, _) = rgb_to_hsl(&hex);
    if s < 15.0 {
        return hex;
    }
    let mut best_distance = f64::MAX;
    let mut best = PASTEL_BACKGROUNDS[0];
    for candidate in PASTEL_BACKGROUNDS {
        let (candidate_hue, _, _) = rgb_to_hsl(candidate);
        let mut distance = (h - candidate_hue).abs();
        if distance > 180.0 {
            distance = 360.0 - distance;
        }
        if distance < best_distance {
            best_distance = distance;
            best = candidate;
        }
    }
    // 色相几乎打平时（例如纯蓝正好落在淡蓝与淡紫中间），再看 RGB 距离，
    // 免得蓝被吸到紫上
    let mut best_rgb = rgb_distance(&hex, best);
    for candidate in PASTEL_BACKGROUNDS {
        let (candidate_hue, _, _) = rgb_to_hsl(candidate);
        let mut distance = (h - candidate_hue).abs();
        if distance > 180.0 {
            distance = 360.0 - distance;
        }
        if distance <= best_distance + 8.0 {
            let rgb = rgb_distance(&hex, candidate);
            if rgb < best_rgb {
                best_rgb = rgb;
                best = candidate;
            }
        }
    }
    best.into()
}

/// 颜色统一成 #RRGGBB，非法值回退到 #000000
pub fn normalize_color(value: &str) -> String {
    let v = value.trim();
    if v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return v.to_uppercase();
    }
    if v.len() == 4 && v.starts_with('#') {
        let b: Vec<char> = v[1..].chars().collect();
        if b.iter().all(|c| c.is_ascii_hexdigit()) {
            return format!(
                "#{}{}{}{}{}{}",
                b[0], b[0], b[1], b[1], b[2], b[2]
            )
            .to_uppercase();
        }
    }
    "#000000".into()
}

/// 全局设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub autostart: bool,
    pub default_layer: Layer,
    pub default_shadow_preset: String,
    pub default_background_color: String,
    pub default_text_color: String,
    pub default_font_family: String,
    pub default_font_size: f64,
    pub default_border_enabled: bool,
    pub default_frame_style: FrameStyle,
    pub global_hotkeys_enabled: bool,
    pub show_updated_at: bool,
    /// 管理窗口上次关闭时的尺寸（逻辑像素）
    pub manager_width: f64,
    pub manager_height: f64,
    /// 管理窗口上次关闭时的位置（逻辑像素）；None 表示还没记录过，保持默认居中
    pub manager_x: Option<f64>,
    pub manager_y: Option<f64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            autostart: false,
            default_layer: Layer::Top,
            default_shadow_preset: "soft".into(),
            default_background_color: "#FFF1A8".into(),
            default_text_color: "#222222".into(),
            default_font_family: "Microsoft YaHei".into(),
            default_font_size: 15.0,
            default_border_enabled: false,
            default_frame_style: FrameStyle::None,
            global_hotkeys_enabled: true,
            show_updated_at: false,
            manager_width: 360.0,
            manager_height: 600.0,
            manager_x: None,
            manager_y: None,
        }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        self.default_font_size = self.default_font_size.clamp(9.0, 40.0);
        self.default_background_color = pastel_background(&self.default_background_color);
        self.default_text_color = normalize_color(&self.default_text_color);
        if self.default_shadow_preset.is_empty() {
            self.default_shadow_preset = "soft".into();
        }
        self.manager_width = self.manager_width.clamp(320.0, 2000.0);
        self.manager_height = self.manager_height.clamp(400.0, 2000.0);
        // 位置可能来自旧文件被手工改坏，非有限值一律丢弃，交给默认居中
        self.manager_x = self.manager_x.filter(|v| v.is_finite()).map(|v| v.round());
        self.manager_y = self.manager_y.filter(|v| v.is_finite()).map(|v| v.round());
    }
}

/// 阴影预设
pub fn shadow_preset(name: &str) -> Shadow {
    match name {
        "none" => Shadow {
            enabled: false,
            color: "#000000".into(),
            opacity: 0.0,
            blur: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            preset: "none".into(),
        },
        "hard" => Shadow {
            enabled: true,
            color: "#000000".into(),
            opacity: 0.45,
            blur: 0.0,
            offset_x: 4.0,
            offset_y: 4.0,
            preset: "hard".into(),
        },
        "deep" => Shadow {
            enabled: true,
            color: "#000000".into(),
            opacity: 0.55,
            blur: 32.0,
            offset_x: 0.0,
            offset_y: 12.0,
            preset: "deep".into(),
        },
        "neon" => Shadow {
            enabled: true,
            color: "#FF3FA4".into(),
            opacity: 0.8,
            blur: 32.0,
            offset_x: 0.0,
            offset_y: 0.0,
            preset: "neon".into(),
        },
        // soft 以及未知值都回退到柔和
        _ => Shadow::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_origin(note: &Note) -> (f64, f64) {
        let insets = note.window_insets();
        (note.x + insets.left, note.y + insets.top)
    }

    #[test]
    fn pastel_background_maps_old_saturated_colors() {
        // 旧版的高饱和淡黄底映射到现在的淡黄，不能变成白色
        assert_eq!(pastel_background("#FFF7B2"), "#FFF1A8");
        // 早先那套偏灰的淡色也按色相落到新色板上（#F2EEDE 是米黄 -> 淡黄）
        assert_eq!(pastel_background("#F2EEDE"), "#FFF1A8");
        assert_eq!(pastel_background("#DDEEE6"), "#B7E4C7");
        assert_eq!(pastel_background("#E8EAED"), "#E8EAED"); // 淡灰饱和度过低，保留
        // 纯红/纯蓝这类刺眼色按色相落到对应的淡色，都不是白色
        assert_eq!(pastel_background("#FF0000"), "#FFC8DD");
        assert_eq!(pastel_background("#0000FF"), "#A8D8FF");
        for mapped in PASTEL_BACKGROUNDS {
            assert_eq!(pastel_background(mapped), mapped, "淡色不应被改动");
        }
    }

    #[test]
    fn pastel_background_keeps_neutral_colors() {
        // 黑白灰是无彩色，不去改用户的深浅选择
        assert_eq!(pastel_background("#FFFFFF"), "#FFFFFF");
        assert_eq!(pastel_background("#000000"), "#000000");
        assert_eq!(pastel_background("#7F8C8D"), "#7F8C8D");
    }

    #[test]
    fn frame_switch_never_drifts() {
        let mut note = Note {
            note_type: NoteType::Image,
            x: 100.0,
            y: 80.0,
            width: 320.0,
            height: 240.0,
            shadow: shadow_preset("soft"),
            ..Note::default()
        };
        let origin = content_origin(&note);

        // 来回切换 5 次以上，内容原点必须始终不变
        for style in [
            FrameStyle::Polaroid,
            FrameStyle::None,
            FrameStyle::Rounded,
            FrameStyle::Polaroid,
            FrameStyle::Thin,
            FrameStyle::Gallery,
            FrameStyle::None,
            FrameStyle::Polaroid,
            FrameStyle::None,
        ] {
            let old_insets = note.window_insets();
            note.frame.style = style;
            note.sanitize();
            note.anchor_content_after_insets_change(old_insets);

            let now = content_origin(&note);
            assert!((now.0 - origin.0).abs() < 1e-9, "x drifted: {now:?}");
            assert!((now.1 - origin.1).abs() < 1e-9, "y drifted: {now:?}");
        }
    }

    #[test]
    fn shadow_change_keeps_content_position() {
        let mut note = Note {
            note_type: NoteType::Text,
            x: 50.0,
            y: 60.0,
            width: 280.0,
            height: 200.0,
            ..Note::default()
        };
        let origin = content_origin(&note);

        for preset in ["none", "soft", "hard", "deep", "neon", "none"] {
            let old_insets = note.window_insets();
            note.shadow = shadow_preset(preset);
            note.sanitize();
            note.anchor_content_after_insets_change(old_insets);

            let now = content_origin(&note);
            assert!((now.0 - origin.0).abs() < 1e-9);
            assert!((now.1 - origin.1).abs() < 1e-9);
        }
    }

    #[test]
    fn window_size_adds_frame_insets_only() {
        let mut note = Note {
            note_type: NoteType::Image,
            width: 320.0,
            height: 240.0,
            shadow: shadow_preset("soft"),
            ..Note::default()
        };
        // 阴影不占窗口空间；拍立得左右/上 3、下 44
        note.frame.style = FrameStyle::Polaroid;
        note.frame.width = 3.0;
        note.frame.frame_bottom_width = 44.0;
        let (w, h) = note.window_size();
        assert_eq!(w, 320.0 + 3.0 * 2.0);
        assert_eq!(h, 240.0 + 3.0 + 44.0);
    }

    #[test]
    fn window_matches_note_in_every_layer() {
        let mut note = Note {
            note_type: NoteType::Text,
            width: 260.0,
            height: 180.0,
            shadow: shadow_preset("deep"),
            ..Note::default()
        };
        // 任何层级都不留阴影留白，窗口就是便签本身
        for layer in [Layer::Desktop, Layer::Top, Layer::Normal] {
            note.layer = layer;
            assert_eq!(note.window_size(), (260.0, 180.0));
            assert_eq!(note.window_insets().left, 0.0);
            assert_eq!(note.window_insets().top, 0.0);
        }

        // 相框变大时内容仍锚定在原屏幕位置
        let origin = content_origin(&note);
        let old_insets = note.window_insets();
        note.note_type = NoteType::Image;
        note.frame.style = FrameStyle::Polaroid;
        note.frame.width = 8.0;
        note.frame.frame_bottom_width = 40.0;
        note.anchor_content_after_insets_change(old_insets);
        let now = content_origin(&note);
        assert!((now.0 - origin.0).abs() < 1e-9);
        assert!((now.1 - origin.1).abs() < 1e-9);
    }

    #[test]
    fn text_notes_ignore_image_frame() {
        let note = Note {
            note_type: NoteType::Text,
            frame: Frame {
                style: FrameStyle::Polaroid,
                width: 20.0,
                frame_bottom_width: 100.0,
                ..Frame::default()
            },
            ..Note::default()
        };
        assert_eq!(note.frame_insets().left, 0.0);
        assert_eq!(note.frame_insets().bottom, 0.0);
    }

    #[test]
    fn color_normalization() {
        assert_eq!(normalize_color("#abc"), "#AABBCC");
        assert_eq!(normalize_color(" #12ef09 "), "#12EF09");
        assert_eq!(normalize_color("red"), "#000000");
    }

    #[test]
    fn old_note_json_defaults_category_and_trash() {
        // 旧版 notes.json 没有 category / deletedAt 字段，读出来应是“未分类 + 正常便签”
        let json = r#"{"id":"note-1","type":"text","content":"hi","x":1.0,"y":2.0,"width":280.0,"height":200.0}"#;
        let note: Note = serde_json::from_str(json).unwrap();
        assert_eq!(note.category, "");
        assert!(note.deleted_at.is_none());
        assert!(!note.is_trashed());
    }

    #[test]
    fn category_default_name() {
        assert_eq!(Category::default().name, "未分类");
        assert!(Category::default().id.is_empty());
    }

    #[test]
    fn order_categories_follows_ids_and_keeps_stragglers() {
        let current = vec![
            Category { id: "a".into(), name: "A".into() },
            Category { id: "b".into(), name: "B".into() },
            Category { id: "c".into(), name: "C".into() },
        ];
        // 上移 b：b 在前，其余保持原相对顺序
        let ids = vec!["b".to_string(), "a".to_string(), "c".to_string()];
        let names: Vec<_> = order_categories(&current, &ids)
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert_eq!(names, vec!["B", "A", "C"]);

        // ids 里漏掉的分类补在末尾，不丢
        let partial = vec!["c".to_string()];
        let names: Vec<_> = order_categories(&current, &partial)
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert_eq!(names, vec!["C", "A", "B"]);

        // 重复 / 不存在的 id 都不能造成重复项
        let messy = vec!["c".to_string(), "c".to_string(), "zzz".to_string()];
        let names: Vec<_> = order_categories(&current, &messy)
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert_eq!(names, vec!["C", "A", "B"]);
    }

    #[test]
    fn manager_position_defaults_to_none_and_drops_invalid() {
        // 没记录过位置时保持 None，让窗口走默认居中
        let fresh = Settings::default();
        assert_eq!((fresh.manager_x, fresh.manager_y), (None, None));

        let mut settings = Settings {
            manager_x: Some(120.4),
            manager_y: Some(f64::NAN),
            ..Settings::default()
        };
        settings.sanitize();
        assert_eq!(settings.manager_x, Some(120.0));
        // 非有限值直接丢弃，不会写进 settings.json
        assert_eq!(settings.manager_y, None);
    }

    #[test]
    fn manager_size_clamped() {
        let mut settings = Settings {
            manager_width: 10.0,
            manager_height: 100.0,
            ..Settings::default()
        };
        settings.sanitize();
        assert_eq!((settings.manager_width, settings.manager_height), (320.0, 400.0));

        settings.manager_width = 480.0;
        settings.manager_height = 700.0;
        settings.sanitize();
        assert_eq!((settings.manager_width, settings.manager_height), (480.0, 700.0));

        let fresh = Settings::default();
        assert_eq!((fresh.manager_width, fresh.manager_height), (360.0, 600.0));
    }
}
