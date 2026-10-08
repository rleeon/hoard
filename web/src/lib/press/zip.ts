// A stored (uncompressed) zip. The kit is images that are compressed already,
// so deflate would buy nothing, and this avoids a dependency for ~60 lines.

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(data: Uint8Array): number {
  let c = 0xffffffff;
  for (const byte of data) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

export type ZipEntry = { name: string; data: Uint8Array };

/** `date` stamps every entry; pass a fixed one so the same tree builds the
 *  same bytes. */
export function storedZip(entries: ZipEntry[], date: Date): ArrayBuffer {
  const encoder = new TextEncoder();
  const dosTime = (date.getUTCHours() << 11) | (date.getUTCMinutes() << 5) | (date.getUTCSeconds() >> 1);
  const dosDate = ((date.getUTCFullYear() - 1980) << 9) | ((date.getUTCMonth() + 1) << 5) | date.getUTCDate();

  const prepared = entries.map((e) => ({ name: encoder.encode(e.name), data: e.data, crc: crc32(e.data) }));
  const localSize = prepared.reduce((n, e) => n + 30 + e.name.length + e.data.length, 0);
  const centralSize = prepared.reduce((n, e) => n + 46 + e.name.length, 0);
  const out = new ArrayBuffer(localSize + centralSize + 22);
  const view = new DataView(out);
  const bytes = new Uint8Array(out);

  let at = 0;
  const offsets: number[] = [];
  for (const e of prepared) {
    offsets.push(at);
    view.setUint32(at, 0x04034b50, true);
    view.setUint16(at + 4, 20, true);
    view.setUint16(at + 6, 0x0800, true); // names are UTF-8
    view.setUint16(at + 8, 0, true); // stored
    view.setUint16(at + 10, dosTime, true);
    view.setUint16(at + 12, dosDate, true);
    view.setUint32(at + 14, e.crc, true);
    view.setUint32(at + 18, e.data.length, true);
    view.setUint32(at + 22, e.data.length, true);
    view.setUint16(at + 26, e.name.length, true);
    view.setUint16(at + 28, 0, true);
    bytes.set(e.name, at + 30);
    bytes.set(e.data, at + 30 + e.name.length);
    at += 30 + e.name.length + e.data.length;
  }

  const centralStart = at;
  prepared.forEach((e, i) => {
    view.setUint32(at, 0x02014b50, true);
    view.setUint16(at + 4, 20, true);
    view.setUint16(at + 6, 20, true);
    view.setUint16(at + 8, 0x0800, true);
    view.setUint16(at + 10, 0, true);
    view.setUint16(at + 12, dosTime, true);
    view.setUint16(at + 14, dosDate, true);
    view.setUint32(at + 16, e.crc, true);
    view.setUint32(at + 20, e.data.length, true);
    view.setUint32(at + 24, e.data.length, true);
    view.setUint16(at + 28, e.name.length, true);
    // extra, comment, disk, internal and external attributes stay zero
    view.setUint32(at + 42, offsets[i], true);
    bytes.set(e.name, at + 46);
    at += 46 + e.name.length;
  });

  view.setUint32(at, 0x06054b50, true);
  view.setUint16(at + 8, prepared.length, true);
  view.setUint16(at + 10, prepared.length, true);
  view.setUint32(at + 12, centralSize, true);
  view.setUint32(at + 16, centralStart, true);
  return out;
}
