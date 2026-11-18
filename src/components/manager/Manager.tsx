import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../../api";
import type { Category, Note, Settings } from "../../types";
import type { NoteFilter } from "../../lib/noteFilter";
import { FILTER_ALL, applyFilter, filterStillValid } from "../../lib/noteFilter";
import { noteTitle, truncate } from "../../lib/noteText";
import { ConfirmDialog } from "../ui";
import SettingsView from "../settings/SettingsView";
import { NotesView } from "./NotesView";
import { TrashView } from "./TrashView";
import { StyleView } from "./StyleView";
import { TabPill } from "./TabPill";

const IMAGE_EXT = /\.(png|jpe?g|webp|gif)$/i;

export default function Manager() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [view, setView] = useState<"notes" | "trash" | "settings" | "style">("notes");
  const [styleId, setStyleId] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Note | null>(null);
  const [purgeTarget, setPurgeTarget] = useState<Note | null>(null);
  const [query, setQuery] = useState("");
  // 一整排只有一个选中项：全部 / 文字便签 / 图片便签 / 未分类 / 某个自建分类
  const [filter, setFilter] = useState<NoteFilter>(FILTER_ALL);
  const [batch, setBatch] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  // 窗口隐藏期间后端广播的 notes-changed 先记账、不刷新；重新显示后补拉一次
  const hiddenRefresh = useRef(false);

  // 后端每次写盘都会广播 notes-changed，操作后不必再手动拉一遍
  const refresh = useCallback(async () => {
    try {
      setNotes(await api.listNotes());
    } catch (e) {
      setWarning(String(e));
    }
  }, []);

  const refreshCategories = useCallback(async () => {
    try {
      setCategories(await api.listCategories());
    } catch (e) {
      setWarning(String(e));
    }
  }, []);

  // 样式面板就在本窗口里打开，不再单独开窗
  const openStyle = useCallback((id: string) => {
    setBatch(false);
    setSelected([]);
    setStyleId(id);
    setView("style");
  }, []);

  const exitBatch = useCallback(() => {
    setBatch(false);
    setSelected([]);
  }, []);

  const addImages = useCallback(async () => {
    const selectedFiles = await open({
      multiple: true,
      title: "选择图片",
      filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
    });
    if (!selectedFiles) return;
    const paths = Array.isArray(selectedFiles) ? selectedFiles : [selectedFiles];
    for (const path of paths) {
      try {
        await api.importImage(path);
      } catch (e) {
        setWarning(String(e));
      }
    }
  }, []);

  useEffect(() => {
    refresh();
    refreshCategories();
    api.getSettings().then(setSettings).catch(() => {});
    api.takeWarning().then((w) => w && setWarning(w)).catch(() => {});

    const unlisteners: Array<() => void> = [];
    listen("notes-changed", () => {
      // 窗口隐藏时列表没人看，先跳过刷新；重新显示时在 visibilitychange 里补一次
      if (document.visibilityState === "hidden") {
        hiddenRefresh.current = true;
        return;
      }
      refresh();
    }).then((un) => unlisteners.push(un));
    listen("categories-changed", () => refreshCategories()).then((un) =>
      unlisteners.push(un),
    );
    listen("settings-changed", () => {
      api.getSettings().then(setSettings).catch(() => {});
    }).then((un) => unlisteners.push(un));
    listen("add-image-request", () => {
      addImages();
    }).then((un) => unlisteners.push(un));
    listen("open-settings", () => {
      exitBatch();
      setView("settings");
    }).then((un) => unlisteners.push(un));
    listen<string>("open-style-panel", (event) => {
      if (event.payload) openStyle(event.payload);
    }).then((un) => unlisteners.push(un));
    listen<string>("app-warning", (event) => setWarning(event.payload)).then((un) =>
      unlisteners.push(un),
    );

    // 重新显示时补一次刷新，避免隐藏期间漏掉后端广播
    const onVisibility = () => {
      if (document.visibilityState === "visible" && hiddenRefresh.current) {
        hiddenRefresh.current = false;
        refresh();
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    const onPaste = async (event: ClipboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (
        target &&
        (target instanceof HTMLInputElement ||
          target instanceof HTMLTextAreaElement ||
          target.isContentEditable)
      ) {
        return;
      }
      const items = Array.from(event.clipboardData?.items ?? []);
      if (!items.some((item) => item.type.startsWith("image/"))) return;
      event.preventDefault();
      try {
        await api.importImageClipboard();
      } catch (e) {
        setWarning(String(e));
      }
    };
    window.addEventListener("paste", onPaste);

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        const files = event.payload.paths.filter((p) => IMAGE_EXT.test(p));
        if (files.length === 0) return;
        (async () => {
          for (const file of files) {
            try {
              await api.importImage(file);
            } catch (e) {
              setWarning(String(e));
            }
          }
        })();
      })
      .then((un) => unlisteners.push(un))
      .catch(() => {});

    return () => {
      unlisteners.forEach((un) => un());
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("paste", onPaste);
    };
  }, [refresh, refreshCategories, addImages, openStyle, exitBatch]);

  const saveSettings = async (next: Settings) => {
    const saved = await api.saveSettings(next);
    setSettings(saved);
    return saved;
  };

  const activeNotes = useMemo(() => notes.filter((n) => !n.deletedAt), [notes]);
  const trashedNotes = useMemo(
    () =>
      notes
        .filter((n) => !!n.deletedAt)
        .sort((a, b) => Date.parse(b.deletedAt ?? "") - Date.parse(a.deletedAt ?? "")),
    [notes],
  );

  const categoryName = useCallback(
    (id: string) => categories.find((c) => c.id === id)?.name ?? "",
    [categories],
  );

  // 筛选行 + 搜索可以叠加；排序按更新时间从新到旧
  const filtered = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    let list = applyFilter(activeNotes, filter);
    if (keyword) {
      list = list.filter((n) => (n.content || "").toLowerCase().includes(keyword));
    }
    return list
      .slice()
      .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt));
  }, [activeNotes, query, filter]);

  // 分类被删掉后，筛选回到“全部”
  useEffect(() => {
    if (!filterStillValid(filter, categories.map((c) => c.id))) setFilter(FILTER_ALL);
  }, [categories, filter]);

  const selectedSet = useMemo(() => new Set(selected), [selected]);

  const changeView = (next: "notes" | "trash" | "settings") => {
    if (next !== "notes") exitBatch();
    setView(next);
  };

  const toggleSelect = useCallback((id: string) => {
    setSelected((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  }, []);

  const selectAll = () => setSelected(filtered.map((n) => n.id));
  const invertSelect = () =>
    setSelected(filtered.filter((n) => !selectedSet.has(n.id)).map((n) => n.id));

  const run = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      setWarning(String(e));
    }
  };

  const batchVisible = (visible: boolean) =>
    run(async () => {
      await api.setNotesVisible(selected, visible);
      setSelected([]);
    });

  const batchLock = (locked: boolean) =>
    run(async () => {
      await api.setNotesLocked(selected, locked);
      setSelected([]);
    });

  const batchTrash = () =>
    run(async () => {
      await api.trashNotes(selected);
      setSelected([]);
    });

  const batchCategory = (value: string) => {
    if (!value) return;
    const category = value === "__none__" ? "" : value;
    run(async () => {
      await api.setNotesCategory(selected, category);
      setSelected([]);
    });
  };

  const styleNote = notes.find((n) => n.id === styleId);
  const styleTitle = !styleNote ? "便签" : truncate(noteTitle(styleNote), 16);

  const hasFilter = !(filter.kind === "all") || query.trim().length > 0;

  const activeTab = view === "trash" ? 1 : view === "settings" ? 2 : 0;

  return (
    <div className="manager">
      <header className="mgr-header">
        <div className="brand">
          <span className="brand-dot" />
          <span>X Notes</span>
        </div>
        <div className="mgr-tools">
          <nav className="tabs">
            <TabPill active={activeTab} />
            <button
              className={activeTab === 0 ? "active" : ""}
              onClick={() => changeView("notes")}
            >
              便签列表
            </button>
            <button
              className={activeTab === 1 ? "active" : ""}
              onClick={() => changeView("trash")}
            >
              回收站
            </button>
            <button
              className={activeTab === 2 ? "active" : ""}
              onClick={() => changeView("settings")}
            >
              设置
            </button>
          </nav>
        </div>
      </header>

      {warning ? (
        <div className="banner">
          <span>{warning}</span>
          <button onClick={() => setWarning(null)}>知道了</button>
        </div>
      ) : null}

      {view === "notes" ? (
        <NotesView
          notes={activeNotes}
          filtered={filtered}
          categories={categories}
          query={query}
          filter={filter}
          batch={batch}
          selected={selected}
          selectedSet={selectedSet}
          hasFilter={hasFilter}
          categoryName={categoryName}
          onQuery={setQuery}
          onFilter={setFilter}
          onCreateText={() => api.createNote("text")}
          onAddImages={addImages}
          onAllVisible={(visible) => run(() => api.setAllVisible(visible))}
          onEnterBatch={() => setBatch(true)}
          onExitBatch={exitBatch}
          onToggleSelect={toggleSelect}
          onSelectAll={selectAll}
          onInvertSelect={invertSelect}
          onDelete={setPendingDelete}
          onStyle={openStyle}
          onBatchVisible={batchVisible}
          onBatchLock={batchLock}
          onBatchTrash={batchTrash}
          onBatchCategory={batchCategory}
        />
      ) : view === "trash" ? (
        <TrashView
          notes={trashedNotes}
          categories={categories}
          categoryName={categoryName}
          onEmpty={() => run(() => api.emptyTrash())}
          onRestore={(note) => run(() => api.restoreNotes([note.id]))}
          onPurge={setPurgeTarget}
        />
      ) : view === "style" && styleId ? (
        <StyleView noteId={styleId} title={styleTitle} onBack={() => setView("notes")} />
      ) : settings ? (
        <SettingsView settings={settings} onSave={saveSettings} />
      ) : (
        <div className="empty">正在读取设置…</div>
      )}

      {pendingDelete ? (
        <ConfirmDialog
          title="移入回收站"
          message={`确定把“${truncate(noteTitle(pendingDelete), 20)}”移进回收站吗？之后可以在回收站里恢复。`}
          confirmLabel="移入回收站"
          onCancel={() => setPendingDelete(null)}
          onConfirm={async () => {
            const target = pendingDelete;
            setPendingDelete(null);
            await run(() => api.deleteNote(target.id));
          }}
        />
      ) : null}

      {purgeTarget ? (
        <ConfirmDialog
          title="彻底删除"
          message={`删除后无法恢复，确定彻底删除“${truncate(noteTitle(purgeTarget), 20)}”吗？`}
          confirmLabel="彻底删除"
          onCancel={() => setPurgeTarget(null)}
          onConfirm={async () => {
            const target = purgeTarget;
            setPurgeTarget(null);
            await run(() => api.purgeNotes([target.id]));
          }}
        />
      ) : null}
    </div>
  );
}
