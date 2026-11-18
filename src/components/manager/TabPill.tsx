import { useLayoutEffect, useRef, useState } from "react";

/**
 * 顶部标签选中态下面的液态玻璃小药丸。
 * 位置和宽度跟着当前选中的标签走，切换时滑过去而不是瞬移。
 */
export function TabPill({ active }: { active: number }) {
  const pillRef = useRef<HTMLSpanElement | null>(null);
  const [box, setBox] = useState<{ left: number; width: number } | null>(null);
  const [ready, setReady] = useState(false);

  useLayoutEffect(() => {
    const pill = pillRef.current;
    const nav = pill?.parentElement;
    if (!nav) return;

    const measure = () => {
      const button = nav.querySelectorAll("button")[active];
      if (!button) return;
      setBox({ left: button.offsetLeft, width: button.offsetWidth });
    };

    measure();
    // 首帧量到位置后再开启过渡，避免从小药丸从左上角滑进来的怪效果
    const timer = window.setTimeout(() => setReady(true), 20);

    const observer = new ResizeObserver(measure);
    observer.observe(nav);
    return () => {
      window.clearTimeout(timer);
      observer.disconnect();
    };
  }, [active]);

  return (
    <span
      ref={pillRef}
      className={`tab-pill${ready ? " ready" : ""}`}
      aria-hidden="true"
      style={box ? { width: box.width, transform: `translateX(${box.left}px)` } : undefined}
    />
  );
}
