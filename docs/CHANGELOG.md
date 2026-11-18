# 变更记录

完整的功能说明见 [README](../README.md)；每次交付产物的 SHA256、测试与验收记录见 [`release/VERSION.txt`](../release/VERSION.txt)。

发布页与下载：[Releases · V1.1.1](https://github.com/scar12138/X_Notes/releases/tag/V1.1.1)；可直接贴到 GitHub Release 正文的说明草稿见 [RELEASE_NOTES_V1.1.1.md](RELEASE_NOTES_V1.1.1.md)。

## 2026-11-17（1.1.1 改名）

- 产品名统一为 **X Notes**（原「桌面便签 / 便签管理」）：窗口标题、主界面品牌、托盘提示与菜单、页面标题全部为 X Notes。
- `tauri.conf.json`：`productName` = `X Notes`、`version` = 1.1.1、`bundle.publisher` = `X Notes`；可执行文件改名为 `x-notes.exe`，安装包为 `X Notes_1.1.1_x64-setup.exe`。exe 属性的 FileDescription / ProductName / CompanyName 现在都是 X Notes（原先 CompanyName 由 identifier 推出，显示为 `stickynotes`）。
- `package.json` / `package-lock.json`：`name` = `x-notes`、版本 1.1.1；`Cargo.toml` 包名 `x-notes`、库名 `x_notes_lib`；单实例互斥体 `Local\XNotes.SingleInstance.v1`；日志前缀 `[x-notes]`。
- 图标（`src-tauri/icons/`）、便签内容、功能与界面样式均未改动。
- `bundle.identifier` 保持 `com.stickynotes.desktop`，保证 `%APPDATA%` 数据目录不搬家。

## 2026-11-17（1.1.0 维护批次 · 第二批）

### 性能

- **列表缩略图**：新增 `read_thumbnail` 命令，在 Rust 侧把图片缩到最长边 144px 再回传 data URL，并按“文件名 + 修改时间”缓存；管理列表不再每次把整张原图 base64 到前端。便签本体仍显示原图。
- **管理窗口隐藏时跳过刷新**：窗口不可见时 `notes-changed` 只记标记，重新显示时补拉一次列表。

### 体验

- **设置页即时保存**：去掉底部“保存设置”按钮；开关/下拉立即落盘，数字与颜色输入 300ms 防抖合并。页脚显示 `修改后立即保存 / 正在保存… / 已保存 HH:MM:SS / 保存失败：…`；保存失败时回退为服务端值并标红，不再假装成功。
- **管理窗口记位置**：`Settings` 新增 `managerX` / `managerY`，关闭与退出时记录、启动时恢复；位置不在任何显示器上时保持默认居中。

### 清理

- 删除 `.settings-footer button` 相关的三个死选择器（设置页已无底部按钮）。

## 2026-11-17（1.1.0 维护批次）

### 修复

- **全部隐藏后无法全部显示**：`Ctrl+Alt+H` 统计可见便签时忽略回收站（回收站里的便签 `visible` 仍为 true），现在只统计未删除的便签。
- **定位隐藏便签时不闪/不聚焦**：定位时若窗口尚未创建，改为建窗时带上 `flash=1`，由新窗口在页面加载完成后自己播高亮，并在 show 后聚焦；不再出现“事件发在窗口创建之前被丢弃”。
- **`update_note` 可绕过业务逻辑**：改为字段白名单，`deletedAt` / `visible` / `createdAt` / `updatedAt` / `id` / `type` / `imageFile` 等只能走各自命令，避免绕过窗口销毁与恢复逻辑。
- **列表行“…”点已展开的按钮关不掉**：全局 `mousedown` 关闭菜单时排除“…”按钮本身，交给按钮 `onClick` 做开关。
- **样式面板滑块偶发回跳**：保存请求加序号，晚到的旧响应不再覆盖新值；未发完的本地修改继续合并进界面。
- **托盘进设置后残留批量状态**：`open-settings` 先退出批量并清空选中。
- **管理窗口尺寸上限不一致**：保存上限从 4000 改为 2000，与读取时的钳制一致。
- **“应用阴影到全部”计数偏大且打乱排序**：只统计未删除便签，纯样式操作不再修改 `updatedAt`。

### 清理

- 删除 0 引用的 CSS 变量（`--panel` / `--line` / `--radius` / `--row-bg` / 旧玻璃变量若干），只保留仍在使用的 `--glass-blur` / `--glass-blur-soft`。
- 删除从未读写的 `Note.monitor` 与未使用的 `Settings.data_version`（旧 JSON 中的多余字段由 serde 忽略）。
- 修复 `Cargo.toml` 注释里的乱码字符。
- README 精简为使用 / 构建 / 数据 / 结构 / 测试 / 限制；详细更新记录移到本文件。

## 2026-11-17（1.1.0）

- **改名 X Notes**：窗口标题、页面标题、主界面品牌、托盘提示、单实例提示、`Cargo.toml` 描述统一。
- **统一按钮与开关**：白底药丸按钮（32px / 999px / 淡阴影）、设置页 44×24 开关、状态文字在左；删除类统一 `#D92D20`，悬停 `#B42318`。
- **卡片分层（iOS 设置页风格）**：实色浅灰底 `#F2F2F7` + 纯白卡片 `#FFFFFF`（12px 圆角、1px `#E5E5EA` 边、淡阴影、12px 间隙）；选中态淡蓝边 `#007AFF` + 淡蓝底 `#EAF4FF`。
- **顶部归纳成三行**：标题/页签、搜索 + 「＋ 新建」下拉、一整排筛选药丸（全部 / 文字 / 图片 / 未分类 / 自建分类），「⋯」收着全部显示/隐藏与批量管理。
- **筛选用单值模型**：新增 `NoteFilter`（全部 / 类型 / 分类），与搜索叠加；分类被删除时自动回到“全部”。
- **回收站支持同一排筛选**；「一键清空」为红色药丸按钮。
- **页签滑动药丸**：`TabPill` 测量标签位置，240ms 缓动切换。
- **便签背景换色相分明的 6 色淡色板**；旧背景按色相映射（无彩色保留）。
- **图片便签缩放修复**：宽高按原图比例一起变，`object-fit: contain` 不裁切；窗口取整误差四边均分。
- **修复定位高亮打到所有窗口**：`flash` 监听带窗口 label 注册。

## 2026-11-16（1.0.0）

- 拆分 `Manager` 为 `components/manager/*`，设置页拆到 `components/settings/*`，公共工具抽到 `lib/`。
- 样式拆分为 `styles/base.css`、`note.css`、`manager.css`、`settings.css`、`style-panel.css`、`overlay.css`。
- 修复 `Esc`/失焦退出编辑后显示不更新。
- 分类管理从筛选行移到设置页，新增分类排序；删除分类时便签回到“未分类”。
- 新增回收站（`deletedAt`）与批量管理；数据迁移到 `notes.json` 的 `{ dataVersion, notes, categories }` 格式。
