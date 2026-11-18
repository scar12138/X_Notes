# X Notes V1.1.1

Windows 桌面便签：文字和图片便签都能钉在桌面上。数据全部保存在本机，不联网、不上传云端。

本版是**改名版本**，功能与界面样式与 1.1.0 完全一致。

## 本版变化

- 产品名统一为 **X Notes**：窗口标题、主界面品牌、托盘提示与菜单、安装包名称全部一致（原为「桌面便签 / 便签管理 / StickyNotes」混用）。
- 可执行文件改为 `x-notes.exe`，安装包为 `X Notes_1.1.1_x64-setup.exe`，exe 属性中的 ProductName / CompanyName / FileDescription 均为 X Notes。
- `bundle.identifier` 仍保留 `com.stickynotes.desktop`，**升级不会搬动数据目录**，已有便签、分类、设置和图片原样可见。
- 图标、便签内容、功能、界面样式均未改动。

## 下载

| 文件 | 大小 | 用途 |
| --- | --- | --- |
| `X.Notes-1.1.1-x64-setup.exe` | 1,797,876 B | NSIS 安装包，直接覆盖安装即可升级 |
| `X.Notes-1.1.1-portable.zip` | 2,239,215 B | 便携版，解压后运行 `x-notes.exe` |
| `X.Notes-1.1.1-source.zip` | 163,626 B | 源码快照（不含 `release/`、`.gitignore`、`.gitattributes`） |

> 发布页上的文件名空格被 GitHub 替换成了点（`X.Notes-…`）；本地打包时的名称是 `X Notes-…`，两者内容完全相同。

校验值（SHA256）：

```text
94FBEEA115923009124988C05FD48EF7D6F51F020A9DAE2FB085661B5AE7707A  X.Notes-1.1.1-portable.zip
8117AE8B6ADD409D645A35DA53AA331C31DD4EA22C7DAF260A503B53F318B833  X.Notes-1.1.1-x64-setup.exe
A7D3C62996A61D9241F05B09DDF173A8D474C9C21CF2D3A044F8239FCCDB4B4F  X.Notes-1.1.1-source.zip
```

## 安装与升级

- **安装版**：运行 `X Notes_1.1.1_x64-setup.exe` 覆盖安装，数据目录不变。卸载/升级前建议先在管理窗口「设置 → 导出全部数据」备份一次。
- **便携版**：解压到任意新目录运行 `x-notes.exe`；`WebView2Loader.dll` 必须与 exe 同目录。旧的 `sticky-notes.exe` 可删除。
- 数据位置：`%APPDATA%\com.stickynotes.desktop\`（`notes.json`、`settings.json`、`images\`）。
- 如果便携版放在 `D:\...` 这类目录下启动失败（WebView2 报 `0x800700AA`），是该目录的完整性标签/ACL 问题，不是程序缺陷；把目录移出或按 README 的 `icacls` 命令修复即可。

## 系统要求

- Windows 10 / 11 x64
- Microsoft Edge WebView2 Runtime（安装包使用在线引导安装，也可提前安装）

## 从源码构建

需要 Node.js 18+、Rust stable 与 Tauri 2 的 Windows 构建依赖（Visual Studio Build Tools C++ 桌面开发）：

```powershell
npm install
npm run tauri build
```

`npm run build` 只构建前端；`cd src-tauri && cargo test --release --features custom-protocol` 跑 16 个 Rust 单测。详见 [README](https://github.com/scar12138/X_Notes/blob/main/README.md)。

## 已知限制

- 仅支持 Windows；桌面层级依赖 Windows 桌面窗口结构，部分第三方桌面软件可能不兼容。
- 全局快捷键可能被占用，可在设置中关闭，改用托盘或管理窗口。
- 关闭管理窗口是隐藏到托盘，真正退出请用托盘菜单「退出」。
- 同一用户只允许运行一个实例；重复启动会提示从托盘打开已有实例。
