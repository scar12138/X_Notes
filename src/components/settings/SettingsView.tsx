import { useEffect, useRef, useState } from "react";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import * as api from "../../api";
import type { FrameStyle, Layer, Settings } from "../../types";
import { FRAME_STYLES, FRAME_LABELS, FONT_OPTIONS, LAYER_LABELS, SHADOW_PRESETS } from "../../types";
import { ColorField, ConfirmButton, Field, Section, Toggle } from "../ui";
import { CategorySection } from "./CategorySection";

/** 连续输入（数字框、调色板拖拽）合并成一次保存；开关和下拉框立刻落盘 */
const DEBOUNCE_MS = 300;

export default function SettingsView({
  settings,
  onSave,
}: {
  settings: Settings;
  onSave: (next: Settings) => Promise<Settings>;
}) {
  const [current, setCurrent] = useState<Settings>(settings);
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [statusText, setStatusText] = useState("");
  const [dataDir, setDataDir] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // 即时保存：current 永远是最新值；dirty 表示还有改动没落盘
  const latest = useRef<Settings>(settings);
  const server = useRef<Settings>(settings);
  const saveRef = useRef(onSave);
  const timer = useRef<number | null>(null);
  const dirty = useRef(false);
  const inflight = useRef(0);
  const seq = useRef(0);
  saveRef.current = onSave;
  server.current = settings;

  // 外部改动（导入数据包等）只在本地没有待保存内容时同步过来，避免冲掉正在输入的值
  useEffect(() => {
    if (dirty.current || inflight.current > 0) return;
    latest.current = settings;
    setCurrent(settings);
  }, [settings]);

  useEffect(() => {
    api.getDataDir().then(setDataDir).catch(() => {});
  }, []);

  // 切走设置页时把还在防抖窗口里的改动补写一次，不丢设置
  useEffect(() => {
    return () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current);
        timer.current = null;
      }
      if (dirty.current) {
        saveRef.current(latest.current).catch(() => {});
      }
    };
  }, []);

  async function flush() {
    timer.current = null;
    if (inflight.current > 0) return;
    // 没有新改动就不写盘（防抖定时器可能在保存完成后才到点）
    if (!dirty.current) return;
    const target = latest.current;
    dirty.current = false;
    const mySeq = ++seq.current;
    inflight.current += 1;
    setStatus("saving");
    try {
      const saved = await saveRef.current(target);
      if (mySeq === seq.current) {
        // 后端会夹值、映射淡色，以返回值为准；等待期间用户又改了就不覆盖
        if (!dirty.current) {
          latest.current = saved;
          setCurrent(saved);
        }
        setStatus("saved");
        setStatusText(`已保存 ${new Date().toLocaleTimeString()}`);
      }
    } catch (e) {
      if (mySeq === seq.current) {
        // 失败时拉回服务端真实值，界面不显示假的“已保存”
        const fallback = server.current;
        latest.current = fallback;
        setCurrent(fallback);
        setStatus("error");
        setStatusText(String(e));
      }
    } finally {
      inflight.current -= 1;
      // 保存期间又改了：紧接着再写一次
      if (dirty.current && mySeq === seq.current) {
        timer.current = window.setTimeout(() => {
          void flush();
        }, 0);
      }
    }
  }

  const schedule = (delay: number) => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      void flush();
    }, delay);
  };

  const update = (patch: Partial<Settings>, delay = DEBOUNCE_MS) => {
    const next = { ...latest.current, ...patch };
    latest.current = next;
    setCurrent(next);
    dirty.current = true;
    setStatus("idle");
    schedule(delay);
  };

  const handleExport = async () => {
    try {
      const dest = await saveDialog({
        title: "导出全部便签数据",
        defaultPath: "便签数据备份.zip",
        filters: [{ name: "便签数据包", extensions: ["zip"] }],
      });
      if (!dest) return;
      const count = await api.exportAllData(dest);
      setMessage(`已导出 ${count} 个文件到：${dest}`);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleImport = async () => {
    try {
      const src = await open({
        title: "导入便签数据包",
        multiple: false,
        filters: [{ name: "便签数据包", extensions: ["zip"] }],
      });
      if (!src || Array.isArray(src)) return;
      const count = await api.importAllData(src);
      setMessage(`已导入 ${count} 个文件，便签已重新加载。`);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="settings-view">
      <div className="settings-scroll">
        <Section
          title="启动与快捷键"
          desc="开机自启默认关闭；开启后启动时只恢复便签和托盘图标，不弹管理窗口。"
        >
          <Field label="开机自启">
            <Toggle
              checked={current.autostart}
              onChange={(v) => update({ autostart: v }, 0)}
            />
          </Field>
          <Field
            label="全局快捷键"
            hint="Ctrl+Alt+N 新建文字便签，Ctrl+Alt+H 全部显示/隐藏"
          >
            <Toggle
              checked={current.globalHotkeysEnabled}
              onChange={(v) => update({ globalHotkeysEnabled: v }, 0)}
            />
          </Field>
          <Field label="显示最后修改时间" hint="默认关闭">
            <Toggle
              checked={current.showUpdatedAt}
              onChange={(v) => update({ showUpdatedAt: v }, 0)}
            />
          </Field>
        </Section>

        <Section title="新便签默认样式" desc="只影响之后新建的便签，已有便签不会跟着变。">
          <Field label="默认层级">
            <select
              value={current.defaultLayer}
              onChange={(e) => update({ defaultLayer: e.target.value as Layer }, 0)}
            >
              {(Object.keys(LAYER_LABELS) as Layer[]).map((layer) => (
                <option key={layer} value={layer}>
                  {LAYER_LABELS[layer]}
                </option>
              ))}
            </select>
          </Field>
          <Field label="默认阴影预设">
            <select
              value={current.defaultShadowPreset}
              onChange={(e) => update({ defaultShadowPreset: e.target.value }, 0)}
            >
              {SHADOW_PRESETS.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.name}
                </option>
              ))}
            </select>
          </Field>
          <ColorField
            label="默认背景颜色"
            value={current.defaultBackgroundColor}
            pastel
            onChange={(v) => update({ defaultBackgroundColor: v })}
          />
          <ColorField
            label="默认文字颜色"
            value={current.defaultTextColor}
            onChange={(v) => update({ defaultTextColor: v })}
          />
          <Field label="默认字体">
            <select
              value={current.defaultFontFamily}
              onChange={(e) => update({ defaultFontFamily: e.target.value }, 0)}
            >
              {FONT_OPTIONS.map((font) => (
                <option key={font} value={font} style={{ fontFamily: font }}>
                  {font}
                </option>
              ))}
            </select>
          </Field>
          <Field label="默认字号">
            <input
              type="number"
              min={9}
              max={40}
              value={current.defaultFontSize}
              onChange={(e) => update({ defaultFontSize: Number(e.target.value) })}
            />
          </Field>
          <Field label="新文字便签默认显示边框" hint="默认关闭（无边框）">
            <Toggle
              checked={current.defaultBorderEnabled}
              onChange={(v) => update({ defaultBorderEnabled: v }, 0)}
            />
          </Field>
          <Field label="新图片便签默认相框" hint="默认无边框">
            <select
              value={current.defaultFrameStyle}
              onChange={(e) => update({ defaultFrameStyle: e.target.value as FrameStyle }, 0)}
            >
              {FRAME_STYLES.map((style) => (
                <option key={style} value={style}>
                  {FRAME_LABELS[style]}
                </option>
              ))}
            </select>
          </Field>
        </Section>

        <CategorySection />

        <Section
          title="图片格式"
          desc="PNG 透明通道原样保留；GIF 动图按原图播放动画（不拆帧）。"
        >
          <p className="note-hint">
            JPG / JPEG、WebP、GIF 都可以通过文件选择、拖拽或 Ctrl+V 粘贴导入。导入时会复制一份到数据目录，原图删掉不影响便签。
          </p>
        </Section>

        <Section
          title="数据与备份"
          desc="所有便签和图片都保存在本机这个文件夹里，不上传网络。"
        >
          <Field label="数据文件夹">
            <span className="mono path-text" title={dataDir}>
              {dataDir || "读取中…"}
            </span>
          </Field>
          <div className="row-actions">
            <button onClick={() => api.openDataFolder()}>打开数据文件夹</button>
            <button onClick={handleExport}>导出全部数据…</button>
            <ConfirmButton
              label="导入数据包…"
              confirmLabel="确认覆盖当前数据？"
              onConfirm={handleImport}
            />
          </div>
          <p className="note-hint">
            导出会把 notes.json、settings.json 和 images 里的图片打包成一个 zip。导入前会自动把当前
            notes.json 备份为 notes.json.pre-import。
          </p>
          {message ? <p className="ok-text">{message}</p> : null}
          {error ? <p className="error-text">{error}</p> : null}
        </Section>
      </div>

      <footer className="settings-footer">
        <span className={`dirty-hint${status === "error" ? " is-error" : ""}`}>
          {status === "saving"
            ? "正在保存…"
            : status === "error"
              ? `保存失败：${statusText}`
              : status === "saved"
                ? statusText
                : "修改后立即保存"}
        </span>
      </footer>
    </div>
  );
}
