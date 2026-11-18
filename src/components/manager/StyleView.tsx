import StylePanel from "../StylePanel";

export interface StyleViewProps {
  noteId: string;
  title: string;
  onBack: () => void;
}

/** 样式视图：在管理窗口内嵌样式面板，不再单独开窗 */
export function StyleView({ noteId, title, onBack }: StyleViewProps) {
  return (
    <div className="style-view">
      <div className="style-head">
        <button className="back-button" onClick={onBack}>
          ← 返回
        </button>
        <span className="style-head-title" title={title}>
          样式 · {title}
        </span>
      </div>
      <StylePanel key={noteId} noteId={noteId} onClose={onBack} />
    </div>
  );
}
