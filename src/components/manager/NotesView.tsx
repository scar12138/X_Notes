import { useEffect, useRef, useState } from "react";
import type { Category, Note } from "../../types";
import type { NoteFilter } from "../../lib/noteFilter";
import { FILTER_ALL } from "../../lib/noteFilter";
import { NoteRow } from "./NoteRow";
import { BatchBar } from "./BatchBar";

export interface NotesViewProps {
  notes: Note[];
  filtered: Note[];
  categories: Category[];
  query: string;
  filter: NoteFilter;
  batch: boolean;
  selected: string[];
  selectedSet: Set<string>;
  hasFilter: boolean;
  categoryName: (id: string) => string;
  onQuery: (value: string) => void;
  onFilter: (filter: NoteFilter) => void;
  onCreateText: () => void;
  onAddImages: () => void;
  onAllVisible: (visible: boolean) => void;
  onEnterBatch: () => void;
  onExitBatch: () => void;
  onToggleSelect: (id: string) => void;
  onSelectAll: () => void;
  onInvertSelect: () => void;
  onDelete: (note: Note) => void;
  onStyle: (id: string) => void;
  onBatchVisible: (visible: boolean) => void;
  onBatchLock: (locked: boolean) => void;
  onBatchTrash: () => void;
  onBatchCategory: (value: string) => void;
}

/**
 * 便签列表视图，顶部归纳成三行：
 *   第一行（在 Manager 里）：标题 + 便签列表/回收站/设置
 *   第二行：搜索框 + 「＋ 新建」（点开有文字便签和图片便签两个选项）
 *   第三行：一整排筛选药丸，右侧小按钮收着全部显示/全部隐藏/批量管理
 */
export function NotesView({
  notes,
  filtered,
  categories,
  query,
  filter,
  batch,
  selected,
  selectedSet,
  hasFilter,
  categoryName,
  onQuery,
  onFilter,
  onCreateText,
  onAddImages,
  onAllVisible,
  onEnterBatch,
  onExitBatch,
  onToggleSelect,
  onSelectAll,
  onInvertSelect,
  onDelete,
  onStyle,
  onBatchVisible,
  onBatchLock,
  onBatchTrash,
  onBatchCategory,
}: NotesViewProps) {
  const [newOpen, setNewOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const newRef = useRef<HTMLDivElement>(null);
  const moreRef = useRef<HTMLDivElement>(null);

  // 点别处 / 按 Esc 收起两个小菜单
  useEffect(() => {
    if (!newOpen && !moreOpen) return;
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (newOpen && !newRef.current?.contains(target)) setNewOpen(false);
      if (moreOpen && !moreRef.current?.contains(target)) setMoreOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setNewOpen(false);
      setMoreOpen(false);
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [newOpen, moreOpen]);

  return (
    <div className="notes-view">
      <div className="search-row">
        <input
          className="search-input"
          type="search"
          value={query}
          placeholder="搜索便签内容…"
          onChange={(event) => onQuery(event.target.value)}
        />
        <div className="new-wrap" ref={newRef}>
          <button
            className="btn-primary new-button"
            onClick={() => setNewOpen((v) => !v)}
            aria-expanded={newOpen}
          >
            ＋ 新建
          </button>
          {newOpen ? (
            <div className="new-menu">
              <button
                onClick={() => {
                  setNewOpen(false);
                  onCreateText();
                }}
              >
                新建文字便签
              </button>
              <button
                onClick={() => {
                  setNewOpen(false);
                  onAddImages();
                }}
              >
                新建图片便签
              </button>
            </div>
          ) : null}
        </div>
      </div>

      {/* 一整排筛选：全部 / 文字便签 / 图片便签 / 未分类 / 自建分类，可横向滚动不换行 */}
      <div className="filter-row">
        <div className="filter-chips">
          <button
            className={`cat-chip${filter.kind === "all" ? " on" : ""}`}
            onClick={() => onFilter(FILTER_ALL)}
          >
            全部
          </button>
          <button
            className={`cat-chip${filter.kind === "type" && filter.value === "text" ? " on" : ""}`}
            onClick={() => onFilter({ kind: "type", value: "text" })}
          >
            文字便签
          </button>
          <button
            className={`cat-chip${filter.kind === "type" && filter.value === "image" ? " on" : ""}`}
            onClick={() => onFilter({ kind: "type", value: "image" })}
          >
            图片便签
          </button>
          <button
            className={`cat-chip${filter.kind === "category" && filter.value === "" ? " on" : ""}`}
            onClick={() => onFilter({ kind: "category", value: "" })}
          >
            未分类
          </button>
          {categories.map((cat) => (
            <button
              key={cat.id}
              className={`cat-chip${
                filter.kind === "category" && filter.value === cat.id ? " on" : ""
              }`}
              onClick={() => onFilter({ kind: "category", value: cat.id })}
            >
              {cat.name}
            </button>
          ))}
        </div>

        <div className="more-wrap" ref={moreRef}>
          <button
            className={`more-button${batch ? " on" : ""}`}
            onClick={() => setMoreOpen((v) => !v)}
            aria-expanded={moreOpen}
            title="全部显示 / 全部隐藏 / 批量管理"
          >
            ⋯
          </button>
          {moreOpen ? (
            <div className="more-menu">
              <button
                onClick={() => {
                  setMoreOpen(false);
                  onAllVisible(true);
                }}
              >
                全部显示
              </button>
              <button
                onClick={() => {
                  setMoreOpen(false);
                  onAllVisible(false);
                }}
              >
                全部隐藏
              </button>
              <button
                className={batch ? "on" : ""}
                onClick={() => {
                  setMoreOpen(false);
                  if (batch) onExitBatch();
                  else onEnterBatch();
                }}
              >
                {batch ? "退出批量" : "批量管理"}
              </button>
            </div>
          ) : null}
        </div>
      </div>

      <div className="note-list">
        {notes.length === 0 ? (
          <div className="empty">
            <div className="empty-icon">🗒️</div>
            <p>还没有便签。点“＋ 新建”选文字便签，或把图片拖进来。</p>
          </div>
        ) : filtered.length === 0 ? (
          <div className="empty">
            <div className="empty-icon">🔍</div>
            <p>{hasFilter ? "这个范围内没有便签。" : "没有便签。"}</p>
          </div>
        ) : (
          filtered.map((note) => (
            <NoteRow
              key={note.id}
              note={note}
              categoryName={categoryName(note.category)}
              batch={batch}
              checked={selectedSet.has(note.id)}
              onCheck={onToggleSelect}
              onDelete={onDelete}
              onStyle={onStyle}
            />
          ))
        )}
      </div>

      {batch ? (
        <BatchBar
          selectCount={selected.length}
          categories={categories}
          onSelectAll={onSelectAll}
          onInvertSelect={onInvertSelect}
          onExit={onExitBatch}
          onVisible={onBatchVisible}
          onLock={onBatchLock}
          onTrash={onBatchTrash}
          onCategory={onBatchCategory}
        />
      ) : null}
    </div>
  );
}
