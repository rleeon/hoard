/**
 * Global handle for the "Liberar espacio" dialog.
 *
 * The dialog used to live *only* inside the Account page, behind a banner the
 * user had to go looking for. That's exactly backwards: the moment it's needed
 * is the moment an upload just bounced off a full account, and the user is
 * anywhere but Account, so the way out has to travel to them.
 *
 * Anything that learns the account is full (the `backup_quota_full` feed row,
 * the storage banner, a future native notification) calls `openLiberate()`.
 * `App.svelte` mounts the dialog once, at the shell level, so it can open over
 * whatever screen is up.
 */
import { writable } from "svelte/store";

export const liberateOpen = writable(false);

export function openLiberate() {
  liberateOpen.set(true);
}

export function closeLiberate() {
  liberateOpen.set(false);
}
