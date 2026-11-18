import { memo, useEffect, useRef, useState } from "react";
import * as api from "../../api";
import type { Note } from "../../types";
import { formatTime } from "../../types";
import { noteTitle } from "../../lib/noteText";

export interface NoteRowProps {
  note: Note;
  categoryName: string;
  batch: boolean;
  checked: boolean;
  onCheck: (id: string) => void;
  onDelete: (note: Note) => void;
  onStyle: (id: string) => void;
}

function NoteRowBase({
  note,
  categoryName,
  batch,
  checked,
  onCheck,
  onDelete,
  onStyle,
}: NoteRowProps) {
  const [thumb, setThumb] = useState<string | null>(null);
  const [menuPos, setMenuPos] = useState<{ top: number; right: number } | null>(null);
  const moreRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    let alive = true;
    if (note.type === "image" && note.imageFile) {
      // 列表只要 144px 缩略图：后端缩放并缓存，别每次把整张原图 base64 过来
      api
        .readThumbnail(note.imageFile)
        .then((url) => {
          if (alive) setThumb(url);
        })
        .catch(() => {});
    } else {
      setThumb(null);
    }
    return () => {
      alive = false;
    };
  }, [note.type, note.imageFile]);

  useEffect(() => {
    if (!menuPos) return;
    const close = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      // 点“…”本身交给按钮的 onClick 做开关；这里先关会让它紧接着又打开
      if (moreRef.current?.contains(target)) return;
      if (!target.closest(".row-menu")) setMenuPos(null);
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [menuPos]);

  const openMenu = () => {
    const rect = moreRef.current?.getBoundingClientRect();
    if (!rect) return;
    const right = Math.max(8, window.innerWidth - rect.right);
    const top =
      rect.bottom + 124 > window.innerHeight ? Math.max(8, rect.top - 124) : rect.bottom + 6;
    setMenuPos({ top, right });
  };

  const title = noteTitle(note);

  return (
    <div className={`note-row${batch ? " batch" : ""}${checked ? " checked" : ""}`}>
      {batch ? (
        <input
          type="checkbox"
          className="row-check"
          checked={checked}
          onChange={() => onCheck(note.id)}
        />
      ) : null}

      {note.type === "image" ? (
        <div className="thumb">
          {thumb ? <img src={thumb} alt="" /> : <span>图片</span>}
        </div>
      ) : null}

      <div className="note-main">
        <div className="note-title" title={title}>
          {note.locked ? <span className="lock-mark">🔒</span> : null}
          <span className="note-title-text">{title}</span>
          {categoryName ? <span className="row-cat">{categoryName}</span> : null}
          {!note.visible ? <span className="row-flag">已隐藏</span> : null}
        </div>
        <div className="row-time">更新 {formatTime(note.updatedAt)}</div>
      </div>

      {batch ? null : (
        <div className="row-tools">
          <button onClick={() => api.locateNote(note.id)}>定位</button>
          <button className="danger-ghost" onClick={() => onDelete(note)}>
            删除
          </button>
          <button
            ref={moreRef}
            className={`row-more${menuPos ? " open" : ""}`}
            title="更多操作"
            onClick={() => (menuPos ? setMenuPos(null) : openMenu())}
          >
            …
          </button>
          {menuPos ? (
            <div
              className="row-menu"
              style={{ top: menuPos.top, right: menuPos.right }}
              onClick={() => setMenuPos(null)}
            >
              <button
                onClick={async () => {
                  // 后端写盘后自己会广播 notes-changed，这里不用再手动刷新
                  if (note.visible) await api.hideNote(note.id);
                  else await api.showNote(note.id);
                }}
              >
                {note.visible ? "隐藏便签" : "显示便签"}
              </button>
              <button onClick={() => onStyle(note.id)}>样式设置</button>
              <button
                className={note.clickThrough ? "on" : ""}
                title="穿透后只能从管理窗口或托盘恢复"
                onClick={async () => {
                  await api.setClickThrough(note.id, !note.clickThrough);
                }}
              >
                鼠标穿透{note.clickThrough ? "（已开）" : ""}
              </button>
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}

/** 列表行：搜索每敲一个字都会重渲父组件，这里 memo 掉未变化的行 */
export const NoteRow = memo(NoteRowBase);
