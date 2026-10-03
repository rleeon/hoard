// "Not now" on a warning card lasts the session, as the cards promise: back on
// the next launch if nothing changed. Held here and not in each card, which the
// Panel mounts afresh on every visit, so the warning came back the moment you
// switched tabs.
type Kind = "links" | "folders" | "mirror";

const dismissed = $state<Record<Kind, Set<string>>>({
  links: new Set(),
  folders: new Set(),
  mirror: new Set(),
});

export function isDismissed(kind: Kind, saveId: string): boolean {
  return dismissed[kind].has(saveId);
}

export function dismiss(kind: Kind, saveId: string): void {
  dismissed[kind] = new Set([...dismissed[kind], saveId]);
}
