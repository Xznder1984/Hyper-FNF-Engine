// Hyper Engine icon generator (zero-dependency Node script).
// Produces the PNG + ICO set Tauri expects under icons/.
// Run: node tools/gen-icons.js
"use strict";
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const OUT = path.join(__dirname, "..", "icons");

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const t = Buffer.from(type, "ascii");
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])), 0);
  return Buffer.concat([len, t, data, crc]);
}

function png(width, height, rgb) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 2; // color type RGB
  const raw = Buffer.alloc(height * (1 + width * 3));
  for (let y = 0; y < height; y++) {
    raw[y * (1 + width * 3)] = 0; // filter: none
    rgb.copy(raw, y * (1 + width * 3) + 1, y * width * 3, (y + 1) * width * 3);
  }
  const idat = zlib.deflateSync(raw, { level: 9 });
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
}

// Draw a simple two-tone "H" motif on a dark rounded field. Original artwork.
function draw(size) {
  const px = Buffer.alloc(size * size * 3);
  const bg = [0x0b, 0x10, 0x21];
  const bar = [0x38, 0xbd, 0xf8]; // sky
  const mid = [0xf4, 0x72, 0xb6]; // pink
  const t = Math.max(2, Math.round(size / 7)); // bar thickness
  const ax = Math.round(size * 0.24);
  const bx = Math.round(size * 0.76) - t;
  const mx = Math.round(size * 0.48); // horizontal bar y
  const c = size / 2;
  const r = size * 0.46;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let col = bg;
      const dx = x + 0.5 - c;
      const dy = y + 0.5 - c;
      const inCircle = dx * dx + dy * dy <= r * r;
      const inV = (x >= ax && x < ax + t) || (x >= bx && x < bx + t);
      const inH = y >= mx && y < mx + t && x >= ax && x < ax + t + size * 0.5;
      if (inCircle) {
        if (inV) col = bar;
        else if (inH) col = mid;
      }
      const o = (y * size + x) * 3;
      px[o] = col[0];
      px[o + 1] = col[1];
      px[o + 2] = col[2];
    }
  }
  return px;
}

function ico(entries) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(entries.length, 4);
  const dir = [];
  const blobs = [];
  let offset = 6 + entries.length * 16;
  for (const e of entries) {
    const d = Buffer.alloc(16);
    d.writeUInt8(e.width >= 256 ? 0 : e.width, 0);
    d.writeUInt8(e.height >= 256 ? 0 : e.height, 1);
    d.writeUInt8(0, 2); // palette
    d.writeUInt8(0, 3); // reserved
    d.writeUInt16LE(1, 4); // planes
    d.writeUInt16LE(32, 6); // bpp
    d.writeUInt32LE(e.data.length, 8);
    d.writeUInt32LE(offset, 12);
    offset += e.data.length;
    dir.push(d);
    blobs.push(e.data);
  }
  return Buffer.concat([header, ...dir, ...blobs]);
}

const sizes = [
  { file: "32x32.png", size: 32 },
  { file: "128x128.png", size: 128 },
  { file: "128x128@2x.png", size: 256 },
  { file: "icon.png", size: 512 },
];

fs.mkdirSync(OUT, { recursive: true });
const pngs = sizes.map((s) => {
  const buf = png(s.size, s.size, draw(s.size));
  fs.writeFileSync(path.join(OUT, s.file), buf);
  return { width: s.size, data: buf };
});
const icoBuf = ico([
  { width: 32, data: png(32, 32, draw(32)) },
  { width: 128, data: png(128, 128, draw(128)) },
  { width: 256, data: png(256, 256, draw(256)) },
]);
fs.writeFileSync(path.join(OUT, "icon.ico"), icoBuf);
console.log("icons written to", OUT, sizes.map((s) => s.file).join(", ") + ", icon.ico");