import React from "react";

export function Section({
  title,
  desc,
  children,
}: {
  title: string;
  desc?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="section">
      <div className="section-head">
        <h3>{title}</h3>
        {desc ? <p>{desc}</p> : null}
      </div>
      <div className="section-body">{children}</div>
    </section>
  );
}

export function Field({
  label,
  hint,
  children,
  className,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
  /** 额外类名，用于个别需要单行对齐的行 */
  className?: string;
}) {
  return (
    <label className={`field${className ? ` ${className}` : ""}`}>
      <span className="field-label">
        {label}
        {hint ? <em>{hint}</em> : null}
      </span>
      <span className="field-control">{children}</span>
    </label>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  /** 状态文字，统一显示在开关左边；不传时用「已开启 / 已关闭」 */
  label?: string;
}) {
  const status = label ?? (checked ? "已开启" : "已关闭");
  return (
    <button
      type="button"
      className={`toggle ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
      aria-pressed={checked}
      aria-label={status}
    >
      <span className="toggle-label">{status}</span>
      <span className="toggle-dot" />
    </button>
  );
}

export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  suffix,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
  onChange: (value: number) => void;
}) {
  return (
    <div className="field slider-field">
      <span className="field-label">
        {label}
        <em>
          {Math.round(value * 100) / 100}
          {suffix ?? ""}
        </em>
      </span>
      <span className="field-control">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
        />
      </span>
    </div>
  );
}

const COLOR_SWATCHES = [
  "#000000",
  "#FFFFFF",
  "#7F8C8D",
  "#E74C3C",
  "#E67E22",
  "#F1C40F",
  "#2ECC71",
  "#3498DB",
  "#9B59B6",
  "#FF3FA4",
];

/** 便签背景专用：明度高、饱和度够，黄绿蓝粉紫橙一眼能分开，配深灰文字不刺眼 */
const PASTEL_SWATCHES = [
  "#FFF1A8", // 淡黄
  "#B7E4C7", // 淡绿
  "#A8D8FF", // 淡蓝
  "#FFC8DD", // 淡粉
  "#D8B4FE", // 淡紫
  "#FFD6A5", // 淡橙
];

export function ColorField({
  label,
  value,
  onChange,
  pastel = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  /** 背景颜色用淡色色板，其余颜色仍用通用色板 */
  pastel?: boolean;
}) {
  const isHex = (v: string) => /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(v.trim());
  const [text, setText] = React.useState(value);
  React.useEffect(() => setText(value), [value]);

  const commit = () => {
    if (isHex(text)) onChange(text.trim().toUpperCase());
    else setText(value);
  };

  const swatches = pastel ? PASTEL_SWATCHES : COLOR_SWATCHES;

  return (
    <div className="field color-field">
      <span className="field-label">{label}</span>
      <div className="color-row">
        <input
          type="color"
          value={isHex(value) ? value : "#000000"}
          onChange={(e) => onChange(e.target.value.toUpperCase())}
        />
        <input
          className="color-text"
          type="text"
          value={text}
          spellCheck={false}
          onChange={(e) => setText(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
      </div>
      <div className="swatches">
        {swatches.map((c) => (
          <button
            key={c}
            type="button"
            className={`swatch ${value.toUpperCase() === c ? "active" : ""}`}
            style={{ background: c }}
            title={c}
            onClick={() => onChange(c)}
          />
        ))}
      </div>
    </div>
  );
}

export function ConfirmButton({
  label,
  confirmLabel = "确认？",
  onConfirm,
  className = "",
}: {
  label: string;
  confirmLabel?: string;
  onConfirm: () => void;
  className?: string;
}) {
  const [armed, setArmed] = React.useState(false);
  React.useEffect(() => {
    if (!armed) return;
    const timer = window.setTimeout(() => setArmed(false), 3000);
    return () => window.clearTimeout(timer);
  }, [armed]);
  return (
    <button
      type="button"
      className={`${className} ${armed ? "danger" : ""}`}
      onClick={() => {
        if (armed) {
          setArmed(false);
          onConfirm();
        } else {
          setArmed(true);
        }
      }}
    >
      {armed ? confirmLabel : label}
    </button>
  );
}

/** 统一的确认弹窗：固定 280px 宽、居中，点遮罩或取消即关闭 */
export function ConfirmDialog({
  title,
  message,
  confirmLabel,
  onConfirm,
  onCancel,
}: {
  title: string;
  message: React.ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-mask" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>{title}</h3>
        <p>{message}</p>
        <div className="modal-actions">
          <button onClick={onCancel}>取消</button>
          <button className="danger" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
