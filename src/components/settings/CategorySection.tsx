import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../../api";
import type { Category } from "../../types";
import { ConfirmButton, Section } from "../ui";

/** 设置页里的分类管理：新建 / 改名 / 删除 / 排序，删除时便签自动回到“未分类” */
export function CategorySection() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");

  const refresh = useCallback(async () => {
    try {
      setCategories(await api.listCategories());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // 便签右键菜单里改分类不会动分类列表，只有增删改才需要重读
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen("categories-changed", () => {
      void refresh();
    }).then((un) => {
      unlisten = un;
    });
    return () => unlisten?.();
  }, [refresh]);

  const create = async () => {
    const value = name.trim();
    if (!value) return;
    try {
      await api.createCategory(value);
      setName("");
      setError(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const rename = async (id: string) => {
    const value = editName.trim();
    if (!value) return;
    try {
      await api.renameCategory(id, value);
      setEditingId(null);
      setError(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const remove = async (id: string) => {
    try {
      await api.deleteCategory(id);
      setError(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  // 上移 / 下移：先把本地顺序换过来，再整份提交给后端，失败就回滚
  const move = async (index: number, delta: number) => {
    const target = index + delta;
    if (target < 0 || target >= categories.length) return;
    const next = [...categories];
    const [moved] = next.splice(index, 1);
    next.splice(target, 0, moved);
    setCategories(next);
    try {
      const saved = await api.reorderCategories(next.map((c) => c.id));
      setCategories(saved);
      setError(null);
    } catch (e) {
      setError(String(e));
      await refresh();
    }
  };

  return (
    <Section
      title="分类管理"
      desc="分类用于给便签归组，主界面顶部的标签条可以按分类筛选。删除分类时，里面的便签自动回到“未分类”，不会被删掉。"
    >
      <div className="cat-create">
        <input
          type="text"
          value={name}
          maxLength={20}
          placeholder="新分类名称"
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void create();
          }}
        />
        <button className="btn-primary" onClick={create} disabled={!name.trim()}>
          新建
        </button>
      </div>

      <div className="cat-list">
        <div className="cat-item">
          <span className="cat-name default-cat">未分类</span>
          <span className="cat-note">默认分类</span>
        </div>
        {categories.map((cat, index) =>
          editingId === cat.id ? (
            <div className="cat-item" key={cat.id}>
              <input
                type="text"
                value={editName}
                maxLength={20}
                autoFocus
                onChange={(e) => setEditName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void rename(cat.id);
                }}
              />
              <button className="mini" onClick={() => void rename(cat.id)}>
                保存
              </button>
              <button className="mini" onClick={() => setEditingId(null)}>
                取消
              </button>
            </div>
          ) : (
            <div className="cat-item" key={cat.id}>
              <span className="cat-name">{cat.name}</span>
              <button
                className="mini"
                title="上移"
                disabled={index === 0}
                onClick={() => void move(index, -1)}
              >
                ↑
              </button>
              <button
                className="mini"
                title="下移"
                disabled={index === categories.length - 1}
                onClick={() => void move(index, 1)}
              >
                ↓
              </button>
              <button
                className="mini"
                onClick={() => {
                  setEditingId(cat.id);
                  setEditName(cat.name);
                }}
              >
                改名
              </button>
              <ConfirmButton
                className="mini danger-ghost"
                label="删除"
                confirmLabel="确认删除？"
                onConfirm={() => void remove(cat.id)}
              />
            </div>
          ),
        )}
        {categories.length === 0 ? (
          <p className="note-hint">还没有自建分类，新建一个就能在主界面顶部筛选。</p>
        ) : null}
      </div>

      {error ? (
        <p className="error-text" onClick={() => setError(null)}>
          {error}
        </p>
      ) : null}
    </Section>
  );
}
