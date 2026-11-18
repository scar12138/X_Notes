// 生成应用图标（便签图标），不依赖任何第三方库。
// 运行：node scripts/gen-icons.mjs
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const iconDir = resolve(root, "src-tauri", "icons");
mkdirSync(iconDir, { recursive: true });

const crcTable = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

function encodePng(width, height, rgba) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0;
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function encodeIco(png, size) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(1, 4);
  const entry = Buffer.alloc(16);
  entry[0] = size >= 256 ? 0 : size;
  entry[1] = size >= 256 ? 0 : size;
  entry[2] = 0;
  entry[3] = 0;
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(22, 12);
  return Buffer.concat([header, entry, png]);
}

function inRoundedRect(x, y, x0, y0, x1, y1, r) {
  if (x < x0 || x > x1 || y < y0 || y > y1) return false;
  const cx = Math.min(Math.max(x, x0 + r), x1 - r);
  const cy = Math.min(Math.max(y, y0 + r), y1 - r);
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}

// 画一张 4 倍超采样的便签图标
function render(size) {
  const S = 4;
  const n = size * S;
  const buf = Buffer.alloc(size * size * 4);
  const m = n * 0.07;
  const r = n * 0.16;
  const x0 = m;
  const y0 = m;
  const x1 = n - m;
  const y1 = n - m;
  const fold = n * 0.3;

  const note = [255, 214, 74];
  const foldColor = [233, 181, 32];
  const line = [120, 96, 24];

  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let rgbaSum = [0, 0, 0, 0];
      for (let sy = 0; sy < S; sy++) {
        for (let sx = 0; sx < S; sx++) {
          const x = px * S + sx;
          const y = py * S + sy;
          if (!inRoundedRect(x, y, x0, y0, x1, y1, r)) continue;

          // 右下角折角
          const isFold = x + y > x1 + y1 - fold;
          let color = note;
          if (isFold) color = foldColor;

          // 三条文字线
          const lx0 = x0 + n * 0.18;
          const lx1 = x1 - n * 0.18;
          const lineH = Math.max(1.2 * S, n * 0.055);
          for (let i = 0; i < 3; i++) {
            const ly = y0 + n * (0.26 + i * 0.17);
            const realX1 = i === 2 ? lx0 + (lx1 - lx0) * 0.6 : lx1;
            if (
              x >= lx0 &&
              x <= realX1 &&
              y >= ly &&
              y <= ly + lineH &&
              !isFold
            ) {
              color = line;
            }
          }

          rgbaSum[0] += color[0];
          rgbaSum[1] += color[1];
          rgbaSum[2] += color[2];
          rgbaSum[3] += 255;
        }
      }
      const count = S * S;
      const i = (py * size + px) * 4;
      const a = rgbaSum[3] / count;
      if (a > 0) {
        const cover = rgbaSum[3] / 255;
        buf[i] = Math.round(rgbaSum[0] / cover);
        buf[i + 1] = Math.round(rgbaSum[1] / cover);
        buf[i + 2] = Math.round(rgbaSum[2] / cover);
        buf[i + 3] = Math.round(a);
      }
    }
  }
  return encodePng(size, size, buf);
}

for (const size of [32, 128, 256, 512]) {
  const png = render(size);
  writeFileSync(resolve(iconDir, `${size}x${size}.png`), png);
  if (size === 128) writeFileSync(resolve(iconDir, "128x128@2x.png"), render(256));
  if (size === 512) writeFileSync(resolve(iconDir, "icon.png"), png);
}

writeFileSync(resolve(iconDir, "icon.ico"), encodeIco(render(256), 256));
console.log("图标已生成：src-tauri/icons");
