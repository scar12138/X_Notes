import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { LogicalSize } from "@tauri-apps/api/dpi";
import * as api from "../api";
import type { Note, Settings } from "../types";
import { exportNoteImage } from "../lib/exportImage";
import {
  formatTime,
  shadowBoxShadow,
  shadowOuterBoxShadow,
  windowInsets,
} from "../types";

function getNoteId(): string {
  const q = new URLSearchParams(window.location.search).get("id");
  if (q) return q;
  return new URLSearchParams(window.location.hash.replace(/^#/, "")).get("id") ?? "";
}



export default function NoteWindow() {
  const id = useMemo(getNoteId, []);
  const [note, setNote] = useState<Note | null>(null);
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [imageError, setImageError] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [hover, setHover] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [caretPoint, setCaretPoint] = useState<{ x: number; y: number } | null>(null);
  const [flash, setFlash] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const noteRef = useRef<Note | null>(null);
  const draftRef = useRef("");
  const editingRef = useRef(false);
  // 最近一次确认写盘的文字：判断“还需要保存吗”只看它，不看本地状态（本地可能已经乐观点亮）
  const savedContentRef = useRef<string | null>(null);
  // 写盘请求是否在途：同一时间只允许一个，否则响应乱序会把新文字盖回旧的
  const savingRef = useRef(false);
  // 待写盘的最新文字：null 表示本地没有未落盘的修改
  const pendingRef = useRef<string | null>(null);
  const saveTimer = useRef<number | null>(null);
  const moveTimer = useRef<number | null>(null);
  const flashTimer = useRef<number | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const mirrorRef = useRef<HTMLDivElement | null>(null);
  const textRef = useRef<HTMLDivElement | null>(null);
  // 图片原始像素尺寸：等比缩放要用原图比例，而不是当前窗口比例
  const imageSizeRef = useRef<{ w: number; h: number } | null>(null);
  const thumbTimer = useRef<number | null>(null);
  const [thumb, setThumb] = useState<{ top: number; height: number } | null>(null);
  const [thumbShown, setThumbShown] = useState(false);

  // 悬浮滚动条：只算滑块位置，不占布局；锁定 / 编辑时一律不显示
  const bumpThumb = () => {
    const el = textRef.current;
    const current = noteRef.current;
    const hidden = !el || !current || current.locked || editingRef.current;
    if (hidden || el.scrollHeight - el.clientHeight < 2) {
      setThumb(null);
      setThumbShown(false);
      return;
    }
    const trackH = Math.max(0, el.clientHeight - 12);
    const height = Math.max(20, Math.round((el.clientHeight / el.scrollHeight) * trackH));
    const travel = Math.max(0, trackH - height);
    const maxScroll = Math.max(1, el.scrollHeight - el.clientHeight);
    setThumb({
      top: Math.round((el.scrollTop / maxScroll) * travel),
      height,
    });
    setThumbShown(true);
    if (thumbTimer.current) window.clearTimeout(thumbTimer.current);
    thumbTimer.current = window.setTimeout(() => setThumbShown(false), 1000);
  };

  useEffect(() => () => {
    if (thumbTimer.current) window.clearTimeout(thumbTimer.current);
  }, []);

  useEffect(() => {
    if (!hover || !note || note.locked || editing) {
      if (thumbTimer.current) window.clearTimeout(thumbTimer.current);
      setThumbShown(false);
      return;
    }
    bumpThumb();
    // bumpThumb 只读 DOM 尺寸，不需要进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hover, editing, note?.locked, note?.content, note?.width, note?.height, note?.fontSize, note?.fontFamily]);

  const apply = useCallback((next: Note) => {
    noteRef.current = next;
    // 没有编辑中的文字、也没有在途写盘时，盘上内容就是这份，记下来当判等基准
    if (!editingRef.current && !savingRef.current && pendingRef.current === null) {
      savedContentRef.current = next.content;
    }
    setNote(next);
  }, []);

  useEffect(() => {
    if (!id) return;
    api
      .getNote(id)
      .then(apply)
      .catch((e) => setError(String(e)));
    api.getSettings().then(setSettings).catch(() => {});
  }, [id, apply]);

  useEffect(() => {
    if (note?.type !== "image" || !note.imageFile) {
      setImageSrc(null);
      setImageError(null);
      return;
    }
    let alive = true;
    setImageError(null);
    api
      .readImage(note.imageFile)
      .then((url) => {
        if (!alive) return;
        setImageSrc(url);
        // 记下原图像素尺寸：等比缩放要按原图比例算，不能用当前窗口比例（一旦破过一次就一路错下去）
        const probe = new Image();
        probe.onload = () => {
          if (!alive) return;
          const w = probe.naturalWidth;
          const h = probe.naturalHeight;
          if (w > 0 && h > 0) imageSizeRef.current = { w, h };
        };
        probe.src = url;
      })
      .catch(() => {
        if (alive) {
          setImageSrc(null);
          setImageError("图片文件缺失，请从管理窗口重新导入");
        }
      });
    return () => {
      alive = false;
    };
  }, [note?.type, note?.imageFile]);

  // 统一写盘：同一时间只有一个请求在途；在途期间又改了文字就等这一次写完再补一次，
  // 保证最后落盘的一定是最新文字（顺序反了会把新文字盖回旧的）。
  const flushContent = useCallback(async (): Promise<void> => {
    if (savingRef.current) return;
    const current = noteRef.current;
    if (!current) return;
    if (savedContentRef.current === null) savedContentRef.current = current.content;
    savingRef.current = true;
    try {
      // 每轮取一次“当前最新文字”，写到它等于已落盘内容为止
      for (;;) {
        const target = pendingRef.current ?? noteRef.current?.content ?? "";
        if (target === savedContentRef.current) break;
        const updated = await api.updateNote(current.id, { content: target });
        savedContentRef.current = updated.content;
        // 落盘成功后显示层读最新状态：把服务端返回的内容合回本地。
        // 但若本地又攒了更新文字（退出编辑那一刻改了），这次响应已经落后，不能拿它盖回去。
        const queued = pendingRef.current;
        const behind = queued !== null && queued !== updated.content;
        if (!behind && noteRef.current?.id === updated.id) {
          const next = {
            ...noteRef.current,
            content: updated.content,
            updatedAt: updated.updatedAt,
          };
          noteRef.current = next;
          setNote(next);
        }
        if (pendingRef.current === target) pendingRef.current = null;
      }
    } catch (e) {
      // 写盘失败就把本轮文字留着，下次输入 / 提交时再试，不要静默丢掉
      setError(String(e));
    } finally {
      savingRef.current = false;
    }
  }, []);

  // 输入过程中调用：本地状态立刻更新（显示层不等待写盘），写盘做 600ms 防抖
  const saveContent = useCallback(
    (value: string) => {
      pendingRef.current = value;
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        saveTimer.current = null;
        void flushContent();
      }, 600);
    },
    [flushContent],
  );

  // 提交：把编辑框里的文字立刻写回内容并写盘，再退出编辑。Esc 与失焦都走这一条。
  const commitEdit = useCallback(() => {
    if (saveTimer.current) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    const current = noteRef.current;
    if (!current) return;
    const value = draftRef.current;
    pendingRef.current = value;
    // 先更新状态：退出编辑那一刻显示层就是新文字，不等写盘、不等拖动
    if (current.content !== value) {
      const next = { ...current, content: value };
      noteRef.current = next;
      setNote(next);
    }
    void flushContent();
  }, [flushContent]);

  const startEdit = useCallback(() => {
    const current = noteRef.current;
    if (!current || current.locked || current.type !== "text") return;
    savedContentRef.current = current.content;
    pendingRef.current = null;
    setDraft(current.content);
    draftRef.current = current.content;
    editingRef.current = true;
    setEditing(true);
  }, []);

  const endEdit = useCallback(() => {
    if (!editingRef.current) return;
    editingRef.current = false;
    setEditing(false);
    // 先提交保存（含同步更新状态），再退出；失焦与 Esc 共用同一条流程
    commitEdit();
  }, [commitEdit]);

  const exportCurrentImage = useCallback(async () => {
    const current = noteRef.current;
    if (!current?.imageFile) return;
    try {
      await exportNoteImage(current.imageFile);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<Note>("note-updated", (event) => {
      const payload = event.payload;
      if (!payload || payload.id !== id) return;
      if (editingRef.current) {
        // 编辑中只同步样式，不覆盖正在输入的文字
        const merged = {
          ...payload,
          content: noteRef.current?.content ?? payload.content,
        };
        noteRef.current = merged;
        setNote(merged);
        return;
      }
      // 刚退出编辑时，队列里可能还有编辑过程中发出的旧广播：内容一旦落后于本地就别回退，
      // 否则显示层会被“编辑前的旧文字”覆盖。样式照常同步，内容以最新状态为准。
      const local = noteRef.current?.content;
      const stale =
        savingRef.current ||
        pendingRef.current !== null ||
        (local !== undefined && savedContentRef.current !== null && payload.content !== savedContentRef.current && local !== payload.content);
      if (stale && local !== undefined) {
        const merged = { ...payload, content: local };
        noteRef.current = merged;
        setNote(merged);
        return;
      }
      apply(payload);
    }).then((un) => unlisteners.push(un));

    listen("start-edit", () => startEdit()).then((un) => unlisteners.push(un));
    // 定位高亮：先清掉所有高亮，再只给这一张加；2 秒后自动淡出
    const clearFlash = () => {
      if (flashTimer.current) window.clearTimeout(flashTimer.current);
      flashTimer.current = null;
      setFlash(false);
    };
    listen("flash-clear", clearFlash).then((un) => unlisteners.push(un));
    // 从“定位”打开的新窗口带 flash=1：页面加载完成后再闪，避免后端 emit 落在建窗之前
    if (new URLSearchParams(window.location.search).get("flash") === "1") {
      const timer = window.setTimeout(() => {
        setFlash(true);
        flashTimer.current = window.setTimeout(() => {
          flashTimer.current = null;
          setFlash(false);
        }, 2000);
      }, 150);
      unlisteners.push(() => window.clearTimeout(timer));
    }
    // 必须带上自己的窗口 label 注册：不带 target 时监听器会注册成 EventTarget::Any，
    // 而 Tauri 的 match_any_or_filter 对 Any 恒为真，emit_to 的过滤就失效了，
    // 结果所有便签窗口一起闪。带上 label 后只有被 emit_to 点名的那个窗口收到。
    listen("flash", () => {
      clearFlash();
      setFlash(true);
      flashTimer.current = window.setTimeout(() => {
        flashTimer.current = null;
        setFlash(false);
      }, 2000);
    }, { target: getCurrentWindow().label }).then((un) => unlisteners.push(un));
    listen<Settings>("settings-changed", (event) => setSettings(event.payload)).then((un) =>
      unlisteners.push(un),
    );
    listen("image-export-request", () => {
      void exportCurrentImage();
    }).then((un) => unlisteners.push(un));

    return () => {
      unlisteners.forEach((un) => un());
      if (flashTimer.current) window.clearTimeout(flashTimer.current);
    };
  }, [id, apply, startEdit, exportCurrentImage]);

  // 把图片文件直接拖到便签窗口上，也能新建图片便签
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let alive = true;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        const files = event.payload.paths.filter((path) =>
          /\.(png|jpe?g|webp|gif)$/i.test(path),
        );
        if (files.length === 0) return;
        void (async () => {
          for (const file of files) {
            try {
              await api.importImage(file);
            } catch (e) {
              setError(String(e));
            }
          }
        })();
      })
      .then((un) => {
        if (alive) unlisten = un;
        else un();
      })
      .catch(() => {});
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    win
      .onMoved(({ payload }) => {
        if (moveTimer.current) window.clearTimeout(moveTimer.current);
        moveTimer.current = window.setTimeout(async () => {
          const current = noteRef.current;
          if (!current) return;
          const scale = await win.scaleFactor();
          const x = Math.round((payload.x / scale) * 100) / 100;
          const y = Math.round((payload.y / scale) * 100) / 100;
          if (Math.abs(x - current.x) < 1 && Math.abs(y - current.y) < 1) return;
          try {
            const updated = await api.updateNote(current.id, { x, y });
            noteRef.current = { ...current, x: updated.x, y: updated.y };
            setNote((prev) => (prev ? { ...prev, x: updated.x, y: updated.y } : prev));
          } catch {
            /* 拖动过程中的保存失败不打断交互 */
          }
        }, 350);
      })
      .then((un) => {
        unlisten = un;
      });

    return () => {
      unlisten?.();
      if (moveTimer.current) window.clearTimeout(moveTimer.current);
    };
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && editingRef.current) {
        event.preventDefault();
        endEdit();
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        // 手动保存：立刻写盘，不等防抖。只在编辑中用草稿，否则用当前内容，避免拿旧草稿覆盖
        if (saveTimer.current) {
          window.clearTimeout(saveTimer.current);
          saveTimer.current = null;
        }
        const current = noteRef.current;
        if (!current) return;
        const value = editingRef.current ? draftRef.current : current.content;
        pendingRef.current = value;
        if (current.content !== value) {
          const next = { ...current, content: value };
          noteRef.current = next;
          setNote(next);
        }
        void flushContent();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [endEdit, flushContent]);

  useEffect(() => {
    const onPaste = async (event: ClipboardEvent) => {
      if (editingRef.current) return;
      const items = Array.from(event.clipboardData?.items ?? []);
      const hasImage = items.some((item) => item.type.startsWith("image/"));
      if (!hasImage) return;
      event.preventDefault();
      try {
        await api.importImageClipboard();
      } catch (e) {
        setError(String(e));
      }
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

  // 系统接管拖动后 webview 收不到 pointerup，用后续任一鼠标事件解除“拖动中”锁定
  useEffect(() => {
    if (!dragging) return;
    const stop = () => setDragging(false);
    const onMove = (event: MouseEvent) => {
      if (event.buttons === 0) stop();
    };
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
    window.addEventListener("mouseup", stop);
    window.addEventListener("mousedown", stop);
    window.addEventListener("mousemove", onMove);
    window.addEventListener("blur", stop);
    return () => {
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      window.removeEventListener("mouseup", stop);
      window.removeEventListener("mousedown", stop);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("blur", stop);
    };
  }, [dragging]);

  // 双击拖动条后，用与文字同版式的透明镜像把点击点换算成光标位置
  useEffect(() => {
    if (!editing || !caretPoint) return;
    const el = textareaRef.current;
    const mirror = mirrorRef.current;
    if (!el || !mirror) return;
    const doc = document as Document & {
      caretRangeFromPoint?: (x: number, y: number) => Range | null;
    };
    let offset: number | null = null;
    const range = doc.caretRangeFromPoint?.(caretPoint.x, caretPoint.y) ?? null;
    if (range && mirror.contains(range.startContainer)) {
      const probe = document.createRange();
      probe.selectNodeContents(mirror);
      probe.setEnd(range.startContainer, range.startOffset);
      offset = probe.toString().length;
    }
    el.focus();
    const pos = Math.max(0, Math.min(el.value.length, offset ?? el.value.length));
    try {
      el.setSelectionRange(pos, pos);
    } catch {
      /* 个别环境不支持选中区间，退化为默认光标 */
    }
    setCaretPoint(null);
  }, [editing, caretPoint]);

  const beginResize = (event: React.PointerEvent) => {
    const current = noteRef.current;
    if (!current || current.locked) return;
    event.preventDefault();
    event.stopPropagation();
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      /* 部分环境不支持捕获，窗口级监听仍可用 */
    }

    const startX = event.screenX;
    const startY = event.screenY;
    const startW = current.width;
    const startH = current.height;
    const lockAspect = current.type === "image" && current.lockAspect;
    // 图片便签一律按原图宽高比等比缩放。原图尺寸还没读到时退回当前窗口比例（至少宽高联动）。
    const original = imageSizeRef.current;
    const ratio =
      original && original.h > 0
        ? original.w / original.h
        : startW / Math.max(1, startH);
    const insets = windowInsets(current);
    const win = getCurrentWindow();

    // 实时尺寸：mousemove 期间每帧同时更新元素与窗口，松手前不写盘。
    let liveW = Math.round(startW);
    let liveH = Math.round(startH);
    let frame = 0;
    let inFlight = false;
    let sentW = -1;
    let sentH = -1;

    const schedule = () => {
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        apply();
      });
    };

    const apply = () => {
      const w = liveW;
      const h = liveH;
      setNote((prev) =>
        prev && (prev.width !== w || prev.height !== h)
          ? { ...prev, width: w, height: h }
          : prev,
      );
      const cur = noteRef.current;
      if (cur && (cur.width !== w || cur.height !== h)) {
        noteRef.current = { ...cur, width: w, height: h };
      }
      if (inFlight || (w === sentW && h === sentH)) return;
      inFlight = true;
      sentW = w;
      sentH = h;
      win
        .setSize(new LogicalSize(w + insets.left + insets.right, h + insets.top + insets.bottom))
        .catch(() => {
          /* 单帧设置失败不打断缩放 */
        })
        .finally(() => {
          inFlight = false;
          if (liveW !== sentW || liveH !== sentH) schedule();
        });
    };

    // 尺寸边界：图片便签的最小/最大宽按原图比例折算，宽高只一起变。
    // 直接分别夹宽高会把比例夹坏——缩到最小时被压扁、两边被裁就是这么来的。
    const MIN_W = 80;
    const MIN_H = 60;
    const MAX = 3000;
    const minAspectW = Math.max(MIN_W, MIN_H * ratio);
    const maxAspectW = Math.min(MAX, MAX * ratio);

    const move = (e: PointerEvent) => {
      const dx = e.screenX - startX;
      const dy = e.screenY - startY;
      if (lockAspect) {
        // 宽高只能一起变：取位移大的那个方向定宽度，另一边按原图比例反算
        const byWidth = startW + dx;
        const byHeight = (startH + dy) * ratio;
        let w = Math.abs(dx) >= Math.abs(dy) ? byWidth : byHeight;
        w = Math.min(maxAspectW, Math.max(minAspectW, w));
        liveW = Math.round(w);
        liveH = Math.round(w / ratio);
      } else {
        liveW = Math.round(Math.min(MAX, Math.max(MIN_W, startW + dx)));
        liveH = Math.round(Math.min(MAX, Math.max(MIN_H, startH + dy)));
      }
      schedule();
    };

    const finish = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", finish);
      if (frame) {
        cancelAnimationFrame(frame);
        frame = 0;
      }
      // 松手只保存最终尺寸，不再按坐标重算一次
      if (liveW === Math.round(startW) && liveH === Math.round(startH)) return;
      api
        .resizeNote(current.id, liveW, liveH)
        .then((updated) => {
          noteRef.current = updated;
          setNote(updated);
        })
        .catch((e) => setError(String(e)));
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", finish);
  };

  if (!note) {
    return <div className="note-root empty-root" />;
  }

  const image = note.type === "image";
  const insets = windowInsets(note);
  // 文字便签：内阴影；图片便签：外投影打在和图片等大的容器上（见 .image-clip）
  const shadowCss = shadowBoxShadow(note.shadow);
  const imageShadowCss = shadowOuterBoxShadow(note.shadow);
  const frame = note.frame;
  // 细线框保持方角；其余相框使用相框圆角
  const frameRadius =
    image && frame.style !== "none" && frame.style !== "thin" ? frame.cornerRadius : 0;
  // 图片圆角与相框同心：外圆角 - 相框宽度；无相框时用便签圆角
  const imageRadius =
    frame.style === "none" ? note.cornerRadius : Math.max(0, frameRadius - frame.width);
  const barVisible = !note.locked && !editing && (hover || dragging);
  // 创建时间：便签内部右下角的液态玻璃小药丸，只在悬停且未锁定、未编辑时淡入
  const timePill = (
    <div className={`note-time${hover && !note.locked && !editing ? " visible" : ""}`}>
      创建 {formatTime(note.createdAt)}
      {settings?.showUpdatedAt ? ` · 修改 ${formatTime(note.updatedAt)}` : ""}
    </div>
  );

  // 按住小横条并移动超过 3px 才进入系统拖动，避免误触
  const startDrag = (event: React.MouseEvent) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const origin = { x: event.screenX, y: event.screenY };
    const stopWatch = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", stopWatch);
    };
    const onMove = (move: MouseEvent) => {
      if (Math.abs(move.screenX - origin.x) < 3 && Math.abs(move.screenY - origin.y) < 3) return;
      stopWatch();
      setDragging(true);
      if (editingRef.current) endEdit();
      void getCurrentWindow().startDragging();
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", stopWatch);
  };

  // 双击文字 = 进入编辑，并把光标放到点击处
  const editAtPointer = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const current = noteRef.current;
    if (!current || current.locked || current.type !== "text" || editingRef.current) return;
    setCaretPoint({ x: event.clientX, y: event.clientY });
    startEdit();
  };

  const dragBar = (
    <div
      className={`drag-pill-hit${barVisible ? " visible" : ""}`}
      onMouseDown={startDrag}
      onDoubleClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    >
      <div className="drag-pill" aria-hidden="true" />
    </div>
  );

  return (
    <div
      className={`note-root ${flash ? "flash" : ""} ${note.locked ? "locked" : ""}${
        editing ? " editing" : ""
      }${dragging ? " dragging" : ""}`}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onContextMenu={(event) => {
        event.preventDefault();
        void api.showNoteMenu(note.id);
      }}
    >
      {image ? (
        <div
          className={`image-frame frame-${frame.style}`}
          style={{
            paddingTop: insets.top,
            paddingRight: insets.right,
            paddingBottom: insets.bottom,
            paddingLeft: insets.left,
            background: frame.style === "none" ? "transparent" : frame.color,
            borderRadius: frameRadius,
            opacity: note.opacity,
          }}
        >
          {dragBar}
          {/* 阴影容器：与图片严格等大，overflow 隐藏保证四个角不超出图片 */}
          <div
            className="image-clip"
            style={{
              width: note.width,
              height: note.height,
              borderRadius: imageRadius,
              boxShadow: imageShadowCss,
            }}
          >
            {imageSrc ? (
              <img
                src={imageSrc}
                alt=""
                draggable={false}
                style={{ borderRadius: "inherit" }}
              />
            ) : (
              <div className="image-missing" style={{ borderRadius: "inherit" }}>
                {imageError ?? "图片加载中…"}
              </div>
            )}
          </div>
          {timePill}
        </div>
      ) : (
        <div
          className="note-card"
          style={{
            width: note.width,
            height: note.height,
            background: note.backgroundColor,
            opacity: note.opacity,
            borderRadius: note.border.enabled
              ? note.border.cornerRadius
              : note.cornerRadius,
            border: note.border.enabled
              ? `${note.border.width}px solid ${note.border.color}`
              : "none",
            boxShadow: shadowCss,
            color: note.textColor,
            fontFamily: note.fontFamily,
            fontSize: note.fontSize,
            fontWeight: note.bold ? 700 : 400,
            fontStyle: note.italic ? "italic" : "normal",
            textAlign: note.textAlign,
          }}
          onDoubleClick={editAtPointer}
        >
          {dragBar}
          {editing ? (
            <>
              <textarea
                ref={textareaRef}
                className="note-textarea"
                value={draft}
                autoFocus
                spellCheck={false}
                style={{
                  color: note.textColor,
                  fontFamily: note.fontFamily,
                  fontSize: note.fontSize,
                  fontWeight: note.bold ? 700 : 400,
                  fontStyle: note.italic ? "italic" : "normal",
                  textAlign: note.textAlign,
                }}
                onChange={(event) => {
                  setDraft(event.target.value);
                  draftRef.current = event.target.value;
                  saveContent(event.target.value);
                }}
                onBlur={endEdit}
                onKeyDown={(event) => {
                  if (event.key === "Escape") {
                    event.preventDefault();
                    endEdit();
                  }
                }}
              />
              {caretPoint ? (
                <div className="note-text caret-mirror" ref={mirrorRef} aria-hidden="true">
                  {draft}
                </div>
              ) : null}
            </>
          ) : (
              <div className="note-text" ref={textRef} onScroll={bumpThumb}>
                {note.content || (
                  <span className="placeholder">双击编辑…</span>
                )}
              </div>
            )}
            {timePill}
            {thumb ? (
              <div className="scroll-track" aria-hidden="true">
                <div
                  className={`scroll-thumb${thumbShown && !note.locked && !editing ? " shown" : ""}`}
                  style={{ top: thumb.top, height: thumb.height }}
                />
              </div>
            ) : null}
          </div>
        )}

        {!note.locked ? (
        <div
          className="resize-handle"
          style={{ right: 2, bottom: 2 }}
          onPointerDown={beginResize}
          title="拖动改变大小"
        />
      ) : null}

      {error ? (
        <div className="note-error" onClick={() => setError(null)}>
          {error}
        </div>
      ) : null}
    </div>
  );
}
