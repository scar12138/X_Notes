import { useMemo, useState } from "react";
import type { Category, Note } from "../../types";
import { formatTime } from "../../types";
import type { NoteFilter } from "../../lib/noteFilter";
import { FILTER_ALL, applyFilter } from "../../lib/noteFilter";
import { noteTitle } from "../../lib/noteText";
import { ConfirmButton } from "../ui";

export interface TrashViewProps {
  notes: Note[];
  categories: Category[];
  categoryName: (id: string) => string;
  onEmpty: () => void;
  onRestore: (note: Note) => void;
  onPurge: (note: Note) => void;
}

/** 回收站视图：和主列表同一排筛选，可恢复、可彻底删除、可一键清空 */
export function TrashView({
  notes,
  categories,
  categoryName,
  onEmpty,
  onRestore,
  onPurge,
}: TrashViewProps) {
  const [filter, setFilter] = useState<NoteFilter>(FILTER_ALL);

  const visible = useMemo(() => applyFilter(notes, filter), [notes, filter]);

  return (
    <div className="trash-view">
      <div className="trash-head">
        <span>
          回收站 · {visible.length} 张
          {filter.kind !== "all" && visible.length !== notes.length
            ? `（共 ${notes.length} 张）`
            : ""}
        </span>
        <ConfirmButton
          label="一键清空"
          confirmLabel="确认清空？"
          className="mini danger-ghost"
          onConfirm={onEmpty}
        />
      </div>

      <div className="filter-row">
        <div className="filter-chips">
          <button
            className={`cat-chip${filter.kind === "all" ? " on" : ""}`}
            onClick={() => setFilter(FILTER_ALL)}
          >
            全部
          </button>
          <button
            className={`cat-chip${filter.kind === "type" && filter.value === "text" ? " on" : ""}`}
            onClick={() => setFilter({ kind: "type", value: "text" })}
          >
            文字便签
          </button>
          <button
            className={`cat-chip${filter.kind === "type" && filter.value === "image" ? " on" : ""}`}
            onClick={() => setFilter({ kind: "type", value: "image" })}
          >
            图片便签
          </button>
          <button
            className={`cat-chip${filter.kind === "category" && filter.value === "" ? " on" : ""}`}
            onClick={() => setFilter({ kind: "category", value: "" })}
          >
            未分类
          </button>
          {categories.map((cat) => (
            <button
              key={cat.id}
              className={`cat-chip${
                filter.kind === "category" && filter.value === cat.id ? " on" : ""
              }`}
              onClick={() => setFilter({ kind: "category", value: cat.id })}
            >
              {cat.name}
            </button>
          ))}
        </div>
      </div>

      <div className="note-list">
        {notes.length === 0 ? (
          <div className="empty">
            <div className="empty-icon">🗑️</div>
            <p>回收站是空的。删除的便签会先放到这里。</p>
          </div>
        ) : visible.length === 0 ? (
          <div className="empty">
            <div className="empty-icon">🔍</div>
            <p>这个范围内没有便签。</p>
          </div>
        ) : (
          visible.map((note) => (
            <div className="note-row" key={note.id}>
              <div className="note-main">
                <div className="note-title">
                  <span className="note-title-text">{noteTitle(note)}</span>
                  {categoryName(note.category) ? (
                    <span className="row-cat">{categoryName(note.category)}</span>
                  ) : null}
                </div>
                <div className="row-time">
                  删除于 {note.deletedAt ? formatTime(note.deletedAt) : ""}
                </div>
              </div>
              <div className="row-tools">
                <button onClick={() => onRestore(note)}>恢复</button>
                <button className="danger-ghost" onClick={() => onPurge(note)}>
                  彻底删除
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
