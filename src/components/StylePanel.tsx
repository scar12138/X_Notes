import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import type {
  Border,
  Category,
  Frame,
  Layer,
  Note,
  NotePatch,
  Shadow,
} from "../types";
import {
  FRAME_LABELS,
  FRAME_STYLES,
  FONT_OPTIONS,
  LAYER_LABELS,
  presetShadow,
  SHADOW_PRESETS,
} from "../types";
import { exportNoteImage } from "../lib/exportImage";
import { ColorField, ConfirmButton, Field, Section, Slider, Toggle } from "./ui";

export default function StylePanel({
  noteId,
  onClose,
}: {
  noteId: string;
  onClose: () => void;
}) {
  const id = noteId;
  const [note, setNote] = useState<Note | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [allApplied, setAllApplied] = useState<number | null>(null);

  const pending = useRef<NotePatch | null>(null);
  const timer = useRef<number | null>(null);
  const lastLocal = useRef(0);
  // 请求序号：90ms 防抖期间可能有两次请求在途，旧响应晚到时不能把新值覆盖回去
  const flushSeq = useRef(0);

  const flush = async () => {
    const payload = pending.current;
    pending.current = null;
    if (!payload) return;
    const seq = ++flushSeq.current;
    try {
      const updated = await api.updateNote(id, payload);
      if (seq === flushSeq.current) {
        setNote((prev) => (prev ? { ...updated, ...(pending.current ?? {}) } : updated));
      }
    } catch (e) {
      if (seq === flushSeq.current) setError(String(e));
    }
  };

  const update = (patch: NotePatch, immediate = false) => {
    setNote((prev) => (prev ? { ...prev, ...patch } : prev));
    pending.current = { ...(pending.current ?? {}), ...patch };
    lastLocal.current = Date.now();
    if (timer.current) window.clearTimeout(timer.current);
    if (immediate) {
      void flush();
    } else {
      timer.current = window.setTimeout(() => {
        void flush();
      }, 90);
    }
  };

  useEffect(() => {
    if (!id) return;
    api
      .getNote(id)
      .then(setNote)
      .catch((e) => setError(String(e)));

    const loadCategories = () => {
      api.listCategories().then(setCategories).catch(() => {});
    };
    loadCategories();

    const unlisteners: Array<() => void> = [];
    listen("categories-changed", loadCategories).then((un) => unlisteners.push(un));
    listen<Note>("note-updated", (event) => {
      if (event.payload?.id !== id) return;
      if (Date.now() - lastLocal.current < 500) return;
      setNote(event.payload);
    }).then((un) => unlisteners.push(un));
    // 便签在别处被删掉时，面板自动退回列表
    listen<string>("note-deleted", (event) => {
      if (event.payload === id) onClose();
    }).then((un) => unlisteners.push(un));

    return () => {
      unlisteners.forEach((un) => un());
      if (timer.current) window.clearTimeout(timer.current);
      const payload = pending.current;
      pending.current = null;
      if (payload) void api.updateNote(id, payload).catch(() => {});
    };
  }, [id]);

  if (!note) {
    return <div className="style-panel loading">正在读取便签…</div>;
  }

  const isImage = note.type === "image";
  const setShadow = (patch: Partial<Shadow>) =>
    update({ shadow: { ...note.shadow, ...patch, preset: "custom" } });
  const setBorder = (patch: Partial<Border>) =>
    update({ border: { ...note.border, ...patch } });
  const setFrame = (patch: Partial<Frame>) =>
    update({ frame: { ...note.frame, ...patch } });

  return (
    <div className="style-panel">
      <div className="style-scroll">
        <Section
          title="分类"
          desc="每张便签一个分类；管理窗口上方可按分类筛选。"
        >
          <div className="preset-row wrap">
            <button
              className={`chip ${note.category === "" ? "on" : ""}`}
              onClick={() => update({ category: "" }, true)}
            >
              未分类
            </button>
            {categories.map((cat) => (
              <button
                key={cat.id}
                className={`chip ${note.category === cat.id ? "on" : ""}`}
                onClick={() => update({ category: cat.id }, true)}
              >
                {cat.name}
              </button>
            ))}
          </div>
          {categories.length === 0 ? (
            <p className="note-hint">还没有分类，可以在管理窗口的“管理”里新建。</p>
          ) : null}
        </Section>

        <Section
          title="窗口层级"
          desc="每张便签单独设置，切换后立即生效并保存。"
        >
          <div className="layer-choices">
            {(["top", "desktop", "normal"] as Layer[]).map((layer) => (
              <button
                key={layer}
                className={`layer-choice ${note.layer === layer ? "on" : ""}`}
                onClick={() => update({ layer }, true)}
              >
                <strong>{LAYER_LABELS[layer]}</strong>
                <span>
                  {layer === "top"
                    ? "始终在其他窗口前面"
                    : layer === "desktop"
                      ? "贴在桌面，被窗口盖住"
                      : "和普通窗口一起层叠"}
                </span>
              </button>
            ))}
          </div>
          <Field label="锁定便签" hint="锁定后不能拖动、编辑、改大小">
            <Toggle
              checked={note.locked}
              onChange={(v) => update({ locked: v }, true)}
              label={note.locked ? "已锁定" : "未锁定"}
            />
          </Field>
          <Field label="鼠标穿透" hint="可从管理窗口或托盘关闭">
            <Toggle
              checked={note.clickThrough}
              onChange={(v) => update({ clickThrough: v }, true)}
              label={note.clickThrough ? "已穿透" : "关闭"}
            />
          </Field>
        </Section>

        <Section
          title="阴影"
          desc="控制内容投在桌面上的影子。任意颜色、模糊、偏移，不会被窗口边缘裁掉。"
        >
          <Field label="开启阴影">
            <Toggle
              checked={note.shadow.enabled}
              onChange={(v) => setShadow({ enabled: v })}
              label={note.shadow.enabled ? "开" : "关"}
            />
          </Field>

          <div className="preset-row">
            {SHADOW_PRESETS.map((preset) => (
              <button
                key={preset.id}
                className={`chip ${note.shadow.preset === preset.id ? "on" : ""}`}
                onClick={() => update({ shadow: preset.shadow })}
              >
                {preset.name}
              </button>
            ))}
          </div>
          <button
            className="mini"
            onClick={() => update({ shadow: presetShadow("soft") })}
          >
            恢复“柔和”默认
          </button>

          <ColorField
            label="阴影颜色"
            value={note.shadow.color}
            onChange={(v) => setShadow({ color: v })}
          />
          <Slider
            label="阴影透明度"
            value={Math.round(note.shadow.opacity * 100)}
            min={0}
            max={100}
            suffix="%"
            onChange={(v) => setShadow({ opacity: v / 100 })}
          />
          <Slider
            label="模糊半径"
            value={note.shadow.blur}
            min={0}
            max={40}
            suffix=" px"
            onChange={(v) => setShadow({ blur: v })}
          />
          <Slider
            label="水平偏移 X"
            value={note.shadow.offsetX}
            min={-30}
            max={30}
            suffix=" px"
            onChange={(v) => setShadow({ offsetX: v })}
          />
          <Slider
            label="垂直偏移 Y"
            value={note.shadow.offsetY}
            min={-30}
            max={30}
            suffix=" px"
            onChange={(v) => setShadow({ offsetY: v })}
          />

          <div className="row-actions">
            <ConfirmButton
              label="应用阴影到全部便签"
              confirmLabel="确认套用到全部？"
              onConfirm={async () => {
                try {
                  const count = await api.applyShadowToAll(note.shadow);
                  setAllApplied(count);
                } catch (e) {
                  setError(String(e));
                }
              }}
            />
          </div>
          {allApplied !== null ? (
            <p className="ok-text">已应用到 {allApplied} 张便签。</p>
          ) : null}
        </Section>

        <Section
          title={isImage ? "边框 / 相框" : "边框"}
          desc={
            isImage
              ? "相框是贴着图片边缘的一圈，和阴影互相独立、分开保存。"
              : "默认无边框；打开后只增加一圈边线，不影响阴影。"
          }
        >
          {isImage ? (
            <>
              <div className="preset-row wrap">
                {FRAME_STYLES.map((style) => (
                  <button
                    key={style}
                    className={`chip ${note.frame.style === style ? "on" : ""}`}
                    onClick={() => update({ frame: { ...note.frame, style } }, true)}
                  >
                    {FRAME_LABELS[style]}
                  </button>
                ))}
              </div>

              {note.frame.style !== "none" ? (
                <>
                  <ColorField
                    label="相框颜色"
                    value={note.frame.color}
                    onChange={(v) => setFrame({ color: v })}
                  />
                  <Slider
                    label="相框宽度"
                    value={note.frame.width}
                    min={1}
                    max={30}
                    suffix=" px"
                    onChange={(v) => setFrame({ width: v })}
                  />
                  {note.frame.style !== "thin" ? (
                    <Slider
                      label="相框圆角"
                      value={note.frame.cornerRadius}
                      min={0}
                      max={30}
                      suffix=" px"
                      onChange={(v) => setFrame({ cornerRadius: v })}
                    />
                  ) : null}
                  {note.frame.style === "polaroid" ? (
                    <Slider
                      label="拍立得下边宽度"
                      value={note.frame.frameBottomWidth}
                      min={8}
                      max={120}
                      suffix=" px"
                      onChange={(v) => setFrame({ frameBottomWidth: v })}
                    />
                  ) : null}
                </>
              ) : (
                <p className="note-hint">当前为无边框：图片直接钉在桌面上。</p>
              )}

              <Field label="锁定图片比例" hint="拖动改大小时保持原比例">
                <Toggle
                  checked={note.lockAspect}
                  onChange={(v) => update({ lockAspect: v }, true)}
                />
              </Field>
              <div className="row-actions">
                <button onClick={() => api.restoreImageSize(note.id).then(setNote)}>
                  恢复原始大小
                </button>
                <button
                  onClick={async () => {
                    if (!note.imageFile) return;
                    try {
                      await exportNoteImage(note.imageFile);
                    } catch (e) {
                      setError(String(e));
                    }
                  }}
                >
                  另存为原图…
                </button>
              </div>
            </>
          ) : (
            <>
              <Field label="显示边框" hint="默认关闭，关闭后不留白边">
                <Toggle
                  checked={note.border.enabled}
                  onChange={(v) => setBorder({ enabled: v })}
                  label={note.border.enabled ? "开" : "关"}
                />
              </Field>
              {note.border.enabled ? (
                <>
                  <ColorField
                    label="边线颜色"
                    value={note.border.color}
                    onChange={(v) => setBorder({ color: v })}
                  />
                  <Slider
                    label="边线粗细"
                    value={note.border.width}
                    min={1}
                    max={8}
                    suffix=" px"
                    onChange={(v) => setBorder({ width: v })}
                  />
                  <Slider
                    label="圆角半径"
                    value={note.border.cornerRadius}
                    min={0}
                    max={30}
                    suffix=" px"
                    onChange={(v) => setBorder({ cornerRadius: v })}
                  />
                </>
              ) : null}
            </>
          )}
        </Section>

        <Section
          title="内容"
          desc={isImage ? "图片透明度与无相框时的圆角。" : "字体、颜色与透明度。"}
        >
          {!isImage ? (
            <>
              <ColorField
                label="背景颜色"
                value={note.backgroundColor}
                pastel
                onChange={(v) => update({ backgroundColor: v })}
              />
              <ColorField
                label="文字颜色"
                value={note.textColor}
                onChange={(v) => update({ textColor: v })}
              />
              <Field label="字体">
                <select
                  value={note.fontFamily}
                  onChange={(e) => update({ fontFamily: e.target.value }, true)}
                >
                  {FONT_OPTIONS.map((font) => (
                    <option key={font} value={font} style={{ fontFamily: font }}>
                      {font}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="字号">
                <input
                  type="number"
                  min={9}
                  max={72}
                  value={note.fontSize}
                  onChange={(e) => update({ fontSize: Number(e.target.value) })}
                />
              </Field>
              <div className="row-actions">
                <button
                  className={note.bold ? "on" : ""}
                  onClick={() => update({ bold: !note.bold }, true)}
                >
                  <strong>B</strong> 加粗
                </button>
                <button
                  className={note.italic ? "on" : ""}
                  onClick={() => update({ italic: !note.italic }, true)}
                >
                  <em>I</em> 斜体
                </button>
                <button
                  className={note.textAlign === "left" ? "on" : ""}
                  onClick={() => update({ textAlign: "left" }, true)}
                >
                  左对齐
                </button>
                <button
                  className={note.textAlign === "center" ? "on" : ""}
                  onClick={() => update({ textAlign: "center" }, true)}
                >
                  居中
                </button>
              </div>
            </>
          ) : null}
          <Slider
            label={isImage ? "图片不透明度" : "便签不透明度"}
            value={Math.round(note.opacity * 100)}
            min={10}
            max={100}
            suffix="%"
            onChange={(v) => update({ opacity: v / 100 })}
          />
          <Slider
            label={
              isImage
                ? "图片圆角（无相框时生效）"
                : note.border.enabled
                  ? "便签圆角（关闭边框时生效）"
                  : "便签圆角"
            }
            value={note.cornerRadius}
            min={0}
            max={40}
            suffix=" px"
            onChange={(v) => update({ cornerRadius: v })}
          />
        </Section>

        {error ? (
          <p className="error-text" onClick={() => setError(null)}>
            {error}
          </p>
        ) : null}
      </div>

      <footer className="style-footer">
        <button onClick={() => api.duplicateNote(note.id)}>复制便签</button>
        <button
          onClick={async () => {
            await api.hideNote(note.id);
            onClose();
          }}
        >
          隐藏
        </button>
        <ConfirmButton
          label="删除"
          confirmLabel="确认删除？"
          className="danger-ghost"
          onConfirm={async () => {
            await api.deleteNote(note.id);
            onClose();
          }}
        />
        <div className="footer-spacer" />
        <button onClick={onClose}>关闭</button>
      </footer>
    </div>
  );
}
