# X Notes

X Notes 是一个 Windows 便签工具：Tauri 2 + React + TypeScript + Vite + Rust。所有数据保存在本机，不依赖数据库，不上传云端。

**下载**：[GitHub Releases](https://github.com/scar12138/X_Notes/releases) 提供 x64 安装包（NSIS）和便携版；也可以克隆本仓库自行构建。当前版本 **1.1.1**。

变更记录见 [docs/CHANGELOG.md](docs/CHANGELOG.md)；1.1.1 的发布说明与校验值见 [docs/RELEASE_NOTES_V1.1.1.md](docs/RELEASE_NOTES_V1.1.1.md)。

## 功能

- **文字便签**：新建、编辑、复制、删除（二次确认）、创建/更新时间；字体、字号、粗体、斜体、颜色、对齐。
- **图片便签**：文件选择、拖拽、剪贴板粘贴；支持 PNG / JPG / WebP / GIF；等比缩放、恢复原图大小、右键另存原图。
- **外观**：背景色与透明度、图片透明度、圆角、阴影（开关/颜色/透明度/模糊/偏移/预设）、5 种相框（无、细线、圆角、拍立得、画廊）。背景色板为 6 个色相分明的淡色（淡黄 `#FFF1A8`、淡绿 `#B7E4C7`、淡蓝 `#A8D8FF`、淡粉 `#FFC8DD`、淡紫 `#D8B4FE`、淡橙 `#FFD6A5`），默认淡黄、文字深灰 `#222222`；旧版高饱和/偏灰背景读取时按色相自动映射到最接近的淡色。
- **三层级**：置顶、桌面、普通。桌面层在按 `Win+D` 后仍然可见，位于普通窗口下方；资源管理器重启后自动恢复。
- **管理窗口**：默认 360×600（一屏 6 行），可拖动放大（最小 320×400），关闭时记住尺寸。顶部三行——标题 + 便签列表/回收站/设置；搜索框 + 「＋ 新建」（文字/图片两项）；一整排筛选药丸（全部 / 文字便签 / 图片便签 / 未分类 / 自建分类），右侧「⋯」收着全部显示、全部隐藏、批量管理。筛选、搜索可叠加，列表按更新时间从新到旧排列。
- **批量管理**：复选框 + 全选/反选，底部可批量显示、隐藏、锁定、解锁、删除、移动分类；退出后清空选中。
- **分类**：默认“未分类”；新建、改名、删除、排序在“设置 → 分类管理”，删除分类时其中便签回到“未分类”；每张便签一个分类，样式面板与便签右键菜单都能改。
- **回收站**：删除先移入回收站，桌面不再显示；可按文字/图片/分类筛选，按张恢复、彻底删除或一键清空。
- **样式面板**：不单独开窗，直接在管理窗口内切换（列表“…”→样式设置，或便签右键→样式设置）；分类、层级、锁定、穿透、阴影、边框/相框、内容、复制/隐藏/删除都在里面。
- **持久化**：自动保存；临时文件 + 原子替换；保留 `.bak`；数据损坏时尝试自动恢复并提示；支持导出/导入全部数据。
- **拖动**：悬停时便签顶部中间浮出液态玻璃小横条（64×5，外套 80×20 热区），按住即可拖动；平时便签上不显示控件。
- **启动**：可关闭的全局快捷键；开机自启默认关闭，自启时不弹管理窗口；多显示器拔插后自动把便签挪回可见区域。

## 快捷键

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl+Alt+N` | 新建文字便签 |
| `Ctrl+Alt+H` | 全部显示 / 全部隐藏（只统计未删除的便签） |
| `Ctrl+S` | 保存当前编辑 |
| `Esc` | 退出编辑（先保存并刷新显示，再退出） |
| 双击便签文字 | 进入编辑；点便签外面失焦同样先保存再退出 |
| 悬停顶部小横条 | 出现 64×5 拖动条，按住拖动便签 |
| 右键便签 | 编辑 / 锁定 / 层级 / 分类 / 边框 / 相框 / 阴影 / 样式设置 / 鼠标穿透 / 移入回收站 |
| 滚轮 / 悬停 | 内容超高时右侧浮出 4px 细滚动条，停止滚动 1 秒后淡出；锁定后不出现 |

快捷键可在“设置”中关闭；被其他程序占用时启动后会提示，不影响其他功能。

## 运行环境

- Windows 10/11（x64）
- 需要 WebView2 Runtime（安装包使用在线引导安装；也可提前安装 Microsoft Edge WebView2 Runtime）
- 从源码构建需要 Node.js 18+、Rust stable 与 Tauri 2 的 Windows 构建依赖

## 开发运行

```powershell
npm install
npm run tauri dev
```

仅构建前端：`npm run build`

## 生产构建

```powershell
npm run tauri build
```

产物（本机构建，`release/` 下为打包副本，已加入 `.gitignore`，不随仓库分发——下载请用 [Releases](https://github.com/scar12138/X_Notes/releases)）：

- 可执行文件：`src-tauri/target/release/x-notes.exe`
- 安装包：`src-tauri/target/release/bundle/nsis/X Notes_1.1.1_x64-setup.exe`
- 便携压缩包：`release/X Notes-1.1.1-portable.zip`
- 安装包副本：`release/X Notes-1.1.1-x64-setup.exe`
- 版本与校验值：`release/VERSION.txt`
- 便携运行：把 `x-notes.exe` 与 `WebView2Loader.dll` 放在同一目录后运行

> GitHub Releases 会把文件名中的空格替换为点（例如 `X.Notes-1.1.1-portable.zip`），与本地打包名 `X Notes-1.1.1-portable.zip` 是同一份文件。

`WebView2Loader.dll` 必须与 `x-notes.exe` 同目录；`tauri.conf.json` 已把它作为安装包资源带上。

> 生产构建必须启用 `custom-protocol` feature（`npm run tauri build` 会自动启用）。直接 `cargo build --release` 而不加该 feature 时，程序会尝试连接开发服务器 `http://localhost:1420`。

### 本机（无 MSVC）构建备注

当前验证环境使用 TDM-GCC 5.1 与本地 ws2_32 链接补丁：

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;C:\TDM-GCC-64\bin;$env:PATH"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="C:\TDM-GCC-64\bin\gcc.exe"
$env:RUSTFLAGS="-L native=$env:TEMP\mingwpatch -C link-arg=-lws2_32patch"
npm run tauri build
```

普通开发机安装 Visual Studio Build Tools（C++ 桌面开发）后直接 `npm run tauri build` 即可。

## 数据与备份

```text
%APPDATA%\com.stickynotes.desktop\
  notes.json          便签 + 分类
  notes.json.bak      上一份备份
  settings.json       全局设置
  settings.json.bak
  images\             图片副本
```

> 目录名来自 bundle identifier（`com.stickynotes.desktop`）。本次改名特意保留该标识，已有便签和设置无需迁移。

- 保存时先写 `.tmp`，成功后把旧文件复制为 `.bak`，再替换正式文件。
- `notes.json` 损坏时保留 `notes.json.corrupt` 并尝试从 `.bak` 恢复，结果在管理窗口提示。
- 导入数据前会把当前 `notes.json` 备份为 `notes.json.pre-import`。
- 图片复制到 `images\`，原图移动或删除不影响便签显示。

## 尺寸与阴影模型

- `width` / `height` 是内容区尺寸；`x` / `y` 是窗口左上角，也是便签可见边缘的位置。
- 窗口尺寸 = 内容区 + 相框外扩；**阴影不占窗口留白**，窗口与可见便签等大，点便签外一圈直接落到桌面。
- 便签阴影画在内侧（紧贴边缘的柔和暗边），不会被窗口裁掉；图片便签另有一层外投影，套在严格等大的 `.image-clip` 上，四角不会露出方块。
- 图片按原图比例显示（`object-fit: contain`），缩放时宽高一起变，最小尺寸按原图比例折算。
- 切换相框/阴影会按新内边距反推窗口位置，保证内容不漂移。

## 代码结构

```text
src/
  main.tsx / note.tsx      管理窗口 / 便签窗口入口
  api.ts                  后端命令封装
  types.ts                数据类型、预设、尺寸与阴影计算
  styles.css              样式入口（@import）
  styles/                 base / note / manager / settings / style-panel / overlay
  lib/                    noteText / noteFilter / exportImage
  components/
    ui.tsx                Section / Field / Toggle / Slider / ColorField / ConfirmButton / ConfirmDialog
    NoteWindow.tsx        便签窗口：渲染、拖动、缩放、编辑保存、图片、定位高亮
    StylePanel.tsx        样式面板全部内容
    manager/              Manager / NotesView / NoteRow / TrashView / StyleView / BatchBar / TabPill
    settings/             SettingsView / CategorySection

src-tauri/src/
  lib.rs                  应用装配、窗口管理、退出与启动流程
  commands.rs             全部 #[tauri::command]
  models.rs               Note / Category / Settings 与几何、颜色计算
  storage.rs              notes.json / settings.json 原子读写
  menu.rs / tray.rs       右键菜单 / 托盘
  window_layer.rs         置顶 / 桌面 / 普通三层级
  images.rs / export.rs   图片导入导出、全部数据导出导入
```

前端颜色统一走 `base.css` 的变量：`--bg` / `--card` / `--card-line` / `--card-shadow` / `--accent` / `--accent-bg` / `--fill` / `--danger` / `--danger-hover` / `--ink*` / `--hint`；按钮高度、圆角、阴影只在 `base.css` 定义一份。

## 测试

```powershell
npm run build            # 前端类型检查 + 构建
cd src-tauri
cargo test --release --features custom-protocol
```

单元测试覆盖：相框/阴影切换不漂移、窗口尺寸、颜色规范化、旧数据兼容、管理窗口尺寸钳制、分类排序、导出/导入 ZIP 往返。

## 已知限制

- 仅支持 Windows；桌面层级依赖 Windows 桌面窗口结构，部分第三方桌面软件可能不兼容。
- 全局快捷键可能被占用，可在设置中关闭后改用托盘/管理窗口。
- 关闭管理窗口会隐藏到托盘，真正退出请用托盘菜单“退出”。
- 同一用户只允许运行一个实例；重复启动会提示从托盘打开已有实例。

## 许可证

[MIT](LICENSE) © 2026 scar12138

你可以自由使用、修改、分发本项目（包括商用），只需保留版权声明与许可声明；软件按「现状」提供，不附带任何担保。
