# 第一批改动记录（已完成）

日期：2026-11-17
状态：**已完成，tsc / cargo test / 构建 / 打包均通过**
范围（用户确认）：P0 三条 + P1 六条 + README 精简 + 死 CSS + 乱码 + 无用字段。

后续批次（本批未做）：
- 第二批：缩略图 `read_thumbnail`、管理窗口隐藏时跳过刷新（显示时补一次）、设置页即时保存、管理窗口记位置。
- 第三批（留到 1.2）：同步命令改 async、回收站自动清理、搜分类名、排序选项、二次启动激活、关于页、CSP、日志、拆 CSS 入口、vitest。

---

## 一、改动清单（均已写入磁盘）

| # | 文件 | 改动 |
|---|------|------|
| 1 | `src-tauri/src/commands.rs` | `toggle_all_visible` 只统计活动便签：`any(\|n\| n.visible && !n.is_trashed())`。 |
| 2 | `src-tauri/src/commands.rs` | `apply_shadow_to_all` 跳过回收站、不再改 `updatedAt`，返回实际应用数量 `applied`。 |
| 3 | `src-tauri/src/commands.rs` | `merge_patch` 改白名单 `PATCHABLE_FIELDS`（内容/几何/样式/分类/锁定/穿透/层级共 22 项）；`id`/`type`/`imageFile`/`createdAt`/`updatedAt`/`deletedAt`/`visible` 一律忽略。 |
| 4 | `src-tauri/src/lib.rs` | 新增 `open_note_window_ext(app, note, flash)`；`open_note_window` 为 `flash=false` 包装；flash 时 URL 带 `note.html?id=…&flash=1`，show 后 `set_focus()`。 |
| 5 | `src-tauri/src/commands.rs` | 新增 `show_note_internal_ex(app, state, id, flash)`；`show_note_internal` 委托 `flash=false`。 |
| 6 | `src-tauri/src/commands.rs` | `locate_note` 重写：先 `flash-clear`；窗口不存在时带 `flash=1` 建窗并直接返回；窗口已存在才 `emit_to("flash")` + show/置顶/聚焦。 |
| 7 | `src-tauri/src/lib.rs` | `remember_manager_size` 上限 4000 → 2000，与 `Settings::sanitize` 一致。 |
| 8 | `src-tauri/src/models.rs` | 删除 `Note.monitor`、删除 `Settings.data_version`（字段 + Default + sanitize 分支）。`notes.json` 的 `dataVersion: 2` 保留。 |
| 9 | `src/types.ts` | 删除 `Note.monitor` 与 `Settings.dataVersion`。 |
| 10 | `src/components/manager/NoteRow.tsx` | 全局 mousedown 关闭菜单时排除“…”按钮（`moreRef.current?.contains(target)` 直接 return），交给 onClick 做开关。 |
| 11 | `src/components/StylePanel.tsx` | 新增 `flushSeq` 请求序号；只有最新响应写回 `note`，并合并 `pending` 中的本地修改；错误提示同样只在最新请求生效时显示。 |
| 12 | `src/components/manager/Manager.tsx` | `exitBatch` 上移到主 `useEffect` 之前；`open-settings` 监听里先 `exitBatch()` 再 `setView("settings")`；依赖数组追加 `exitBatch`。 |
| 13 | `src/components/NoteWindow.tsx` | 页面加载时若 URL 带 `flash=1`，150ms 后播 2 秒高亮（配合第 4/6 项）；卸载时清理定时器。 |
| 14 | `src-tauri/Cargo.toml` | 修复注释乱码：`缺少这�?feature` → `缺少这个 feature`（以 UTF-8 无 BOM 写回）。 |
| 15 | `src/styles/base.css` | 删除 0 引用变量：`--panel` `--line` `--radius` `--row-bg` `--glass-bg` `--glass-strong` `--glass-weak` `--glass-line` `--glass-edge` `--glass-shadow` `--glass-shadow-soft`；保留 `--glass-blur` / `--glass-blur-soft`。 |
| 16 | `README.md` | 重写精简版（功能 / 快捷键 / 环境 / 构建 / 数据 / 尺寸模型 / 结构 / 测试 / 限制），产物路径更新到 1.1.0。 |
| 17 | `docs/CHANGELOG.md` | 新建，收纳原 README 的更新记录、1.0 功能清单、已验证项，新增本批次条目。 |
| 18 | `release/VERSION.txt` | 新增“1.1.0 维护批次”章节、更新交付产物 SHA256 与构建记录。 |

---

## 二、验证结果

```
npx tsc --noEmit                                                    EXIT 0
cargo test --release --features custom-protocol                     13 passed / 0 failed
npm run tauri build                                                 EXIT 0
```

- `cargo test` 在 PowerShell 下返回退出码 1 属本机已知现象，测试本身全通过。
- 唯一构建告警：TDM-GCC `corrupt .drectve at end of def file`，无害。

## 三、本次产物（2026-11-17 晚）

| 文件 | 大小 (B) | SHA256 |
|------|---------|--------|
| `release\StickyNotes-1.1.0-portable.zip` | 2,214,685 | `F9F593B2C4E6DF4FCB744EA085DF909C1FD6D1B06DDC33B2F548A82C08A89675` |
| `release\StickyNotes-1.1.0-x64-setup.exe` | 1,782,985 | `C72D8A9857ABEEE7968D4C998C538DA654DF165ABD2F99AB76F7A930457EFFDE` |
| `release\StickyNotes-1.1.0-source.zip` | — | 见 `release/VERSION.txt`（该压缩包包含本文件，改完此处需重新打包，故校验值只写在 VERSION.txt） |
| `target\release\sticky-notes.exe` | 4,780,544 | `3262F746AA465D0D6147CC3F64B1D433ECB731F543165EDF39E12CF723729D62` |

## 四、待处理

- `release\StickyNotes-1.1.0-portable\` 里的 `sticky-notes.exe` 与 `WebView2Loader.dll`
  被正在运行的旧实例占用，本批未能覆盖（`README.md` 已更新）。
  **zip 内容是新产物**，退出实例后把 zip 解压到该目录即可同步。
- 用户需自行验收交互：Ctrl+Alt+H 往返、定位隐藏便签、列表“…”开关、
  样式面板滑块连续拖动、托盘进设置后批量状态清空。
