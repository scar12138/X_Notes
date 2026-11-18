import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import * as api from "../api";

/**
 * 把便签原图另存到用户选的位置。
 * 用户取消时静默返回 null；出错抛出去，由调用方决定怎么提示。
 */
export async function exportNoteImage(imageFile: string): Promise<string | null> {
  const ext = imageFile.split(".").pop() ?? "png";
  const dest = await saveDialog({
    title: "另存为原图",
    defaultPath: `便签图片.${ext}`,
  });
  if (!dest) return null;
  await api.exportImage(imageFile, dest);
  return dest;
}
