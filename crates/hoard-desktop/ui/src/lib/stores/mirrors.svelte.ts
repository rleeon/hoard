// A save's second folders on this machine: the same save written by another
// program, like one game played in two emulators. The service keeps them equal
// to the save's own folder (`hoard_agent::mirror`); this is what the Library
// shows of it and the three questions it asks.
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import type { TrackedSave } from "../api";

export type MirrorStatus = "pending" | "in_sync" | "conflict" | "missing";
export type MirrorKeep = "primary" | "mirror";

export type MirrorSides = {
  primary_files: number;
  mirror_files: number;
  primary_newest: string | null;
  mirror_newest: string | null;
};

export type MirrorFolder = {
  path: string;
  status: MirrorStatus;
  /** The answer to a conflict, until the service has carried it out. */
  keep?: MirrorKeep | null;
  sides?: MirrorSides | null;
  copied_at?: string | null;
};

export type MirrorDialog =
  | { kind: "add"; save: TrackedSave; path: string }
  | { kind: "remove"; save: TrackedSave; folder: MirrorFolder }
  | { kind: "resolve"; save: TrackedSave; folder: MirrorFolder };

let bySave = $state<Record<string, MirrorFolder[]>>({});
let dialog = $state<MirrorDialog | null>(null);
let settling: ReturnType<typeof setTimeout> | null = null;

export function mirrorsOf(saveId: string): MirrorFolder[] {
  return bySave[saveId] ?? [];
}

export function mirrorDialog(): MirrorDialog | null {
  return dialog;
}

export function closeMirrorDialog(): void {
  dialog = null;
}

/** Windows spells one folder many ways; the comparison should not care. */
function key(path: string): string {
  const flat = path.replace(/\\/g, "/").replace(/\/+$/, "");
  return /^[a-z]:\//i.test(flat) ? flat.toLowerCase() : flat;
}

/** Whether detection's `path` is already some save's second folder. Such a
 *  folder is not a game of its own any more, and the Library stops offering it. */
export function isMirrorFolder(path: string): boolean {
  const wanted = key(path);
  return Object.values(bySave).some((folders) => folders.some((f) => key(f.path) === wanted));
}

function unsettled(): boolean {
  return Object.values(bySave).some((folders) =>
    folders.some((f) => f.status === "pending" || f.keep != null),
  );
}

/** The service works a couple of seconds behind the store. While a folder is
 *  waiting for it, look again; `tries` keeps a service that is down from being
 *  asked for ever. */
function follow(tries = 20): void {
  if (settling) clearTimeout(settling);
  settling = null;
  if (tries <= 0 || !unsettled()) return;
  settling = setTimeout(async () => {
    settling = null;
    try {
      bySave = await invoke<Record<string, MirrorFolder[]>>("list_save_mirrors");
    } catch {
      return;
    }
    follow(tries - 1);
  }, 2000);
}

/** `chase` is for a look that follows something the user did. The Library's
 *  own slow poll passes `false`: restarting the fast one from it would ask
 *  every two seconds for as long as the service is down. */
export async function refreshMirrors(chase = true): Promise<void> {
  try {
    bySave = await invoke<Record<string, MirrorFolder[]>>("list_save_mirrors");
  } catch {
    // An older core has no such command: no second folders, nothing to show.
    return;
  }
  if (chase) follow();
}

function put(saveId: string, folders: MirrorFolder[]): void {
  const next = { ...bySave };
  if (folders.length) next[saveId] = folders;
  else delete next[saveId];
  bySave = next;
  follow();
}

export async function addMirror(saveId: string, path: string): Promise<void> {
  put(saveId, await invoke<MirrorFolder[]>("add_save_mirror", { saveId, path }));
}

export async function removeMirror(saveId: string, path: string): Promise<void> {
  put(saveId, await invoke<MirrorFolder[]>("remove_save_mirror", { saveId, path }));
}

export async function keepMirror(saveId: string, path: string, keep: MirrorKeep): Promise<void> {
  put(saveId, await invoke<MirrorFolder[]>("keep_save_mirror", { saveId, path, keep }));
}

/** Picks the folder first and asks second: the dialog can then name it. */
export async function askAddMirror(save: TrackedSave, title: string): Promise<void> {
  const picked = await openDialog({ directory: true, multiple: false, title });
  if (typeof picked === "string" && picked.length > 0) {
    dialog = { kind: "add", save, path: picked };
  }
}

export function askRemoveMirror(save: TrackedSave, folder: MirrorFolder): void {
  dialog = { kind: "remove", save, folder };
}

export function askResolveMirror(save: TrackedSave, folder: MirrorFolder): void {
  dialog = { kind: "resolve", save, folder };
}
