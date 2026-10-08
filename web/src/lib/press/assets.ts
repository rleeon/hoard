/**
 * What goes into the press kit, in order. This list is the only thing to edit:
 * `/press` shows these files and the build packs them into
 * `/hoard-press-kit.zip`. `file` is a path under `web/static/`; a missing file
 * fails the build instead of shipping a kit with a hole in it. GIFs and videos
 * (`.mp4`, `.webm`) work too.
 *
 * Kept free of `$lib` imports and browser code: `vite.config.ts` reads it.
 */
export type PressAssetKind = 'main' | 'screenshot' | 'diagram' | 'icon';

export type PressAsset = {
  file: string;
  /** Path inside the zip; named for whoever unpacks it, not for the site. */
  zipName: string;
  kind: PressAssetKind;
  /** i18n key of the caption. */
  label: string;
};

export const PRESS_ASSETS: PressAsset[] = [
  { file: 'WEB.webp', zipName: 'screenshots/hoard-desktop-app.webp', kind: 'main', label: 'press.asset.app' },
  { file: 'CLI.webp', zipName: 'screenshots/hoard-cli.webp', kind: 'screenshot', label: 'press.asset.cli' },
  { file: 'cloud.png', zipName: 'diagrams/hoard-cloud-or-self-hosted.png', kind: 'diagram', label: 'press.asset.diagram' },
  { file: 'icon-512.png', zipName: 'icons/hoard-icon-512.png', kind: 'icon', label: 'press.asset.icon_square' },
  { file: 'icon2.png', zipName: 'icons/hoard-icon-transparent.png', kind: 'icon', label: 'press.asset.icon_mark' }
];

export type PressAssetMeta = PressAsset & {
  bytes: number;
  /** Null for a video, or for a format the build cannot measure. */
  width: number | null;
  height: number | null;
};

export type PressRelease = {
  version: string;
  date: string;
  /** The free-text paragraph under the version heading, as written. */
  summary: string;
  sections: { name: string; items: string[] }[];
};

export type PressBuild = {
  assets: PressAssetMeta[];
  release: PressRelease | null;
};
