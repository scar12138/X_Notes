export interface BatchBarProps {
  selectCount: number;
  categories: Array<{ id: string; name: string }>;
  onSelectAll: () => void;
  onInvertSelect: () => void;
  onExit: () => void;
  onVisible: (visible: boolean) => void;
  onLock: (locked: boolean) => void;
  onTrash: () => void;
  onCategory: (value: string) => void;
}

/** 批量管理底部操作栏 */
export function BatchBar({
  selectCount,
  categories,
  onSelectAll,
  onInvertSelect,
  onExit,
  onVisible,
  onLock,
  onTrash,
  onCategory,
}: BatchBarProps) {
  const empty = selectCount === 0;
  return (
    <div className="batch-bar">
      <div className="batch-line">
        <button className="mini" onClick={onSelectAll}>
          全选
        </button>
        <button className="mini" onClick={onInvertSelect}>
          反选
        </button>
        <span className="batch-count">已选 {selectCount} 张</span>
        <button className="mini" onClick={onExit}>
          完成
        </button>
      </div>
      <div className="batch-line wrap">
        <button className="mini" disabled={empty} onClick={() => onVisible(true)}>
          显示
        </button>
        <button className="mini" disabled={empty} onClick={() => onVisible(false)}>
          隐藏
        </button>
        <button className="mini" disabled={empty} onClick={() => onLock(true)}>
          锁定
        </button>
        <button className="mini" disabled={empty} onClick={() => onLock(false)}>
          解锁
        </button>
        <button className="mini danger-ghost" disabled={empty} onClick={onTrash}>
          删除
        </button>
        <select
          className="batch-cat-select"
          value=""
          disabled={empty}
          onChange={(e) => onCategory(e.target.value)}
        >
          <option value="">移动分类…</option>
          <option value="__none__">未分类</option>
          {categories.map((cat) => (
            <option key={cat.id} value={cat.id}>
              {cat.name}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
