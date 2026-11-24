// ---- grid pages
// The Panel and the Library show their games a page at a time: with 1006 cards
// mounted the Library drew at 10 to 15 fps, with 60 at 60 or more (23-11).
//
// Opening a game's history unmounts the grid, and the way back landed on the
// first page, far from the game the user had opened. The page is kept here,
// with the filters it was chosen under: other filters are another list, and
// they start on its first page.

const kept = new Map<string, { key: string; page: number }>();

export function keptPage(grid: string, key: string): number {
  const k = kept.get(grid);
  return k && k.key === key ? k.page : 0;
}

export function keepPage(grid: string, key: string, page: number): void {
  kept.set(grid, { key, page });
}

export function pageCount(total: number, size: number): number {
  return Math.max(1, Math.ceil(total / size));
}
