<script module lang="ts">
  // The overlay's live scene, kept at MODULE level (not per instance) so it
  // survives the component unmounting: switching from Screen to another view and
  // back leaves the native overlay process running, so its panels have to reappear
  // without entering editor mode just to make the overlay resend the scene.
  // `onMount` rehydrates it when `screen_is_open`, and `onDestroy` dumps it before
  // leaving. It is not persisted to disk: closing the app kills the overlay and
  // `screen_is_open` goes false.
  // A crosshair's parameters (the overlay's SourceRef::Crosshair). `color` is hex
  // #rrggbb and `alpha` is 0..1; the overlay receives 8-bit RGBA.
  export type Ch = {
    style: "cross" | "x" | "dot" | "circle";
    size: number;
    thickness: number;
    gap: number;
    color: string;
    alpha: number;
    dot: boolean;
    outline: boolean;
  };

  export function defaultCh(): Ch {
    return {
      style: "cross",
      size: 48,
      thickness: 3,
      gap: 6,
      color: "#34d399",
      alpha: 240 / 255,
      dot: false,
      outline: true,
    };
  }

  // The sniper scope's parameters (SourceRef::Scope): a lens that magnifies
  // whatever is under the panel.
  /** The button or key that turns the scope on. Buttons are numbered the way the
   *  browser numbers them (`MouseEvent.button`: 0 left, 1 wheel, 2 right, 3 back,
   *  4 forward) and keys by `KeyboardEvent.code`, which is what arrives when they
   *  are captured; the overlay translates to VK or keysym on its side. */
  export type ScBinding =
    | { type: "mouse"; button: number }
    | { type: "key"; code: string };

  /** When the scope shows: `toggle` alternates, `hold` only while held down,
   *  `timed` turns it on for `seconds` and it goes off by itself. */
  export type ScMode = "toggle" | "hold" | "timed";

  /** What the lens aims at: whatever is under it, the centre of the screen, or an
   *  offset point. It decouples *where you look* from *what you see*. */
  export type ScAim =
    | { kind: "under" }
    | { kind: "center" }
    | { kind: "offset"; dx: number; dy: number };

  export type Sc = {
    shape: "circle" | "square";
    zoom: number;
    border: boolean;
    /** Smooth the magnified pixels (bilinear) or leave them hard (nearest). */
    smooth: boolean;
    /** Cruz fina en el centro de la vista ampliada. */
    reticle: boolean;
    aim: ScAim;
    activation: {
      /** `null` = siempre visible (el comportamiento de siempre). */
      binding: ScBinding | null;
      mode: ScMode;
      seconds: number;
    };
  };

  export const ZOOM_MIN = 1;
  export const ZOOM_MAX = 20;

  /**
   * The magnification slider is NOT linear, and that is not a whim: magnification
   * is perceived multiplicatively (×1 to ×2 reads as big a jump as ×10 to ×20), so
   * a linear run from 1 to 20 would cram the whole useful range, ×1 to ×4, into the
   * first 16% of the bar and make it impossible to fine-tune there. With the
   * exponential curve, the same stretch of bar always changes the magnification by
   * the same *proportion*: the middle of the bar lands on ×4.5 and ×1.01 is still
   * reachable.
   */
  const ZOOM_STEPS = 1000;

  export function sliderToZoom(pos: number): number {
    const t = Math.min(1, Math.max(0, pos / ZOOM_STEPS));
    return Math.round(ZOOM_MAX ** t * 100) / 100;
  }

  export function zoomToSlider(zoom: number): number {
    const z = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom || ZOOM_MIN));
    return Math.round((Math.log(z) / Math.log(ZOOM_MAX)) * ZOOM_STEPS);
  }

  /** ×1,01 · ×2,5 · ×12, sin decimales que no aportan nada. */
  export function zoomLabel(zoom: number): string {
    const z = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom || ZOOM_MIN));
    const txt = z < 10 ? z.toFixed(2).replace(/\.?0+$/, "") : z.toFixed(0);
    return `×${txt}`;
  }

  export function defaultSc(): Sc {
    return {
      shape: "circle",
      zoom: 2,
      border: true,
      smooth: true,
      reticle: false,
      aim: { kind: "under" },
      activation: { binding: null, mode: "toggle", seconds: 3 },
    };
  }

  /** A binding's readable name. Mouse buttons are named, not numbered: "Mouse 4"
   *  says considerably more than "button 3". */
  export function bindingLabel(b: ScBinding | null): string {
    if (!b) return tr({ es: "Sin asignar", en: "Unassigned" });
    if (b.type === "key") return b.code;
    switch (b.button) {
      case 0:
        return tr({ es: "Clic izquierdo", en: "Left click" });
      case 1:
        return tr({ es: "Rueda", en: "Middle click" });
      case 2:
        return tr({ es: "Clic derecho", en: "Right click" });
      case 3:
        return tr({ es: "Ratón 4 (atrás)", en: "Mouse 4 (back)" });
      case 4:
        return tr({ es: "Ratón 5 (adelante)", en: "Mouse 5 (forward)" });
      default:
        return tr({ es: `Botón ${b.button}`, en: `Button ${b.button}` });
    }
  }

  export type Panel = {
    id: string;
    // "window" captura una app; "crosshair" dibuja una mirilla procedural;
    // "scope" es la lente de aumento (visor de francotirador).
    kind: "window" | "crosshair" | "scope";
    windowId: string;
    label: string;
    x: number;
    y: number;
    w: number;
    h: number;
    crop: { top: number; right: number; bottom: number; left: number };
    scale: "fill" | "fit";
    z: number;
    // In Game mode the window is click-through (the clicks reach the game). With
    // protected or accelerated video content that can leave it black; setting
    // `passthrough: false` keeps the window's original style (the video composites
    // normally) in exchange for clicks on that panel reaching the app.
    passthrough: boolean;
    // Chromium compatibility mode: instead of placing the real window cropped
    // (whose crop leaves grey in Brave and Discord), it captures the window and
    // composites the crop pixel-perfect, parking the real window off-screen. It can
    // leave video black during playback (Chromium's occlusion).
    compat: boolean;
    // The radius in px of the circular click-through lens that follows the cursor
    // when `passthrough` is on: inside the circle you see and click what is behind
    // the panel; outside it, the panel looks normal.
    passthroughRadius: number;
    // Which screen this panel lives on. `mirror` = drawn on every monitor;
    // otherwise it's drawn only on `monitorId`. Rects are monitor-local.
    monitorId: number;
    mirror: boolean;
    // Siempre presente (los paneles window lo ignoran): evita pelear con el
    // narrowing de null en la plantilla. Manda solo cuando kind="crosshair",
    // y entonces el rect se mantiene w=h=ch.size para que el blit sea 1:1.
    ch: Ch;
    // Ídem para el visor: solo manda cuando kind="scope".
    sc: Sc;
  };

  let savedPanels: Panel[] = [];
</script>

<script lang="ts">
  // hoard-screen launcher + editor. Drives the NATIVE overlay process
  // (`hoard-screen` sidecar), not a Tauri window anymore.
  //
  // Model (v1): the overlay only composites panels and stays click-through
  // (View). Arranging happens here, in the main window, on a scaled preview of
  // the monitor; every change is pushed to the overlay as a `set_scene` line so
  // the result shows live over the desktop/game. Direct manipulation on the
  // overlay itself (grabbing the mouse there) is a later function.
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { primaryMonitor } from "@tauri-apps/api/window";
  import {
    MonitorPlay,
    SquarePlus,
    SquareX,
    RefreshCw,
    Trash2,
    TriangleAlert,
    Maximize,
    Proportions,
    ArrowUp,
    ArrowDown,
    Gamepad2,
    Pencil,
    Crop,
    RotateCcw,
    MousePointerClick,
    Crosshair,
    Locate,
    ZoomIn,
    Layers,
  } from "@lucide/svelte";
  import { tr } from "./lib";

  type WinInfo = { id: string; title: string; app: string; protected: boolean };
  type Monitor = {
    id: number;
    name: string;
    x: number;
    y: number;
    w: number;
    h: number;
    primary: boolean;
  };
  // `type Panel` vive en el `<script module>` de arriba (se comparte con el
  // buffer `savedPanels` que sobrevive al desmontaje).

  // px of the on-screen preview; overlay maps to monitor. Capped to the
  // rendered width of the preview column so a narrow window (min 800px wide)
  // doesn't force a fixed 560px box wider than the space actually available.
  let previewColWidth = $state(560);
  const PREVIEW_W = $derived(Math.min(560, previewColWidth));

  let open = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let windows = $state<WinInfo[]>([]);
  let panels = $state<Panel[]>([]);
  let selectedId = $state<string | null>(null);
  // Game (click-through) vs Edit (overlay grabs input; panels draggable on the
  // overlay itself). Mirrors the overlay; Ctrl+O / Esc flip it there too.
  let editing = $state(false);

  // Physical monitors (from the native overlay's enumeration). The overlay
  // draws one click-through window per monitor; a panel's rect is local to its
  // target monitor's top-left. `activeMonId` is the screen currently shown in
  // the preview / arranged here.
  let monitors = $state<Monitor[]>([]);
  let activeMonId = $state(0);

  const fallbackMon: Monitor = { id: 0, name: "", x: 0, y: 0, w: 1920, h: 1080, primary: true };
  const activeMon = $derived(
    monitors.find((m) => m.id === activeMonId) ?? monitors[0] ?? fallbackMon,
  );
  // The preview's coordinate space is the active monitor's pixel size.
  const mon = $derived({ w: activeMon.w, h: activeMon.h });
  // Panels drawn on the active screen: its own plus any mirrored everywhere.
  const visiblePanels = $derived(
    panels.filter((p) => p.mirror || p.monitorId === activeMonId),
  );

  const previewScale = $derived(PREVIEW_W / mon.w);
  const previewH = $derived(Math.round(mon.h * previewScale));
  const selected = $derived(panels.find((p) => p.id === selectedId) ?? null);

  function monLabel(m: Monitor) {
    const n = monitors.findIndex((x) => x.id === m.id) + 1;
    return `${tr({ es: "Pantalla", en: "Screen" })} ${n}${m.primary ? " ★" : ""}`;
  }

  // hex #rrggbb + opacidad 0..1 → RGBA de 8 bits (el formato del overlay).
  function hexToRgba(hex: string, alpha: number): [number, number, number, number] {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex);
    const v = m ? parseInt(m[1], 16) : 0x34d399;
    const a = Math.round(clamp(alpha, 0, 1) * 255);
    return [(v >> 16) & 255, (v >> 8) & 255, v & 255, a];
  }

  function rgbaToHex(c: number[] | undefined): string {
    const [r, g, b] = c ?? [52, 211, 153];
    const h = (n: number) =>
      Math.max(0, Math.min(255, Math.round(n)))
        .toString(16)
        .padStart(2, "0");
    return `#${h(r)}${h(g)}${h(b)}`;
  }

  function sceneJson() {
    return {
      panels: panels.map((p) => ({
        id: p.id,
        source:
          p.kind === "crosshair"
            ? {
                kind: "crosshair",
                style: p.ch.style,
                size: p.ch.size,
                thickness: p.ch.thickness,
                gap: p.ch.gap,
                color: hexToRgba(p.ch.color, p.ch.alpha),
                dot: p.ch.dot,
                outline: p.ch.outline,
              }
            : p.kind === "scope"
              ? {
                  kind: "scope",
                  shape: p.sc.shape,
                  zoom: p.sc.zoom,
                  border: p.sc.border,
                  smooth: p.sc.smooth,
                  reticle: p.sc.reticle,
                  aim: p.sc.aim,
                  activation: p.sc.activation,
                }
              : { kind: "window", id: p.windowId },
        rect: { x: p.x, y: p.y, w: p.w, h: p.h },
        crop: p.crop,
        scale: p.scale,
        z: p.z,
        passthrough: p.passthrough,
        compat: p.compat,
        passthrough_radius: p.passthroughRadius,
        monitor: p.mirror ? { kind: "all" } : { kind: "monitor", id: p.monitorId },
      })),
    };
  }

  // The last local edit: while it is recent, an incoming scene from the overlay
  // does not overwrite it (the push already carries it; the poll resyncs once things
  // settle).
  let lastPush = 0;

  // Screen's telemetry: what gets placed in here, so we know whether this is used.
  // The backend times the session and editor mode on its own; what it lacks is the
  // vocabulary (what a crosshair is, what a scope is), and that is the only thing
  // that goes up through here. Never a window title or an app name. Best-effort: a
  // failure is not reported to the user as an error.
  function note(action: string, kind?: string) {
    invoke("screen_note", { action, kind: kind ?? null }).catch(() => {});
  }

  async function pushScene() {
    if (!open) return;
    lastPush = Date.now();
    note("edit");
    try {
      await invoke("screen_send", {
        line: JSON.stringify({ type: "set_scene", scene: sceneJson() }),
      });
    } catch (e) {
      error = String(e);
    }
  }

  // Pide al overlay su escena real (responde con un evento `scene`). Es el
  // arreglo del desync: si la app pierde su copia (recarga, evento perdido),
  // el editor vuelve a reflejar lo que hay EN PANTALLA y todo sigue editable.
  function syncScene() {
    if (!open) return;
    invoke("screen_send", { line: '{"type":"get_scene"}' }).catch(() => {});
  }

  async function loadWindows() {
    try {
      const raw = await invoke<string>("screen_list_windows");
      const parsed = JSON.parse(raw || "[]");
      windows = Array.isArray(parsed) ? parsed : [];
    } catch (e) {
      error = String(e);
      windows = [];
    }
  }

  async function setEditor(on: boolean) {
    editing = on;
    if (!open) return;
    try {
      await invoke("screen_send", {
        line: JSON.stringify({ type: "set_editor", editor: on }),
      });
    } catch (e) {
      error = String(e);
    }
  }

  // Scene pushed back by the overlay after an in-overlay drag/resize: adopt the
  // new geometry/z, keeping our human labels. Skipped while dragging here.
  function applyIncomingScene(scene: any) {
    if (drag || cropDrag) return;
    if (Date.now() - lastPush < 1500) return;
    const sp = scene?.panels;
    if (!Array.isArray(sp)) return;
    panels = sp.map((p: any) => {
      const prev = panels.find((q) => q.id === p.id);
      const t = p.monitor;
      const src = p.source ?? {};
      const isCh = src.kind === "crosshair";
      const isSc = src.kind === "scope";
      return {
        id: p.id,
        kind: isCh
          ? ("crosshair" as const)
          : isSc
            ? ("scope" as const)
            : ("window" as const),
        windowId: isCh || isSc ? "" : (src.id ?? ""),
        label:
          prev?.label ??
          (isCh
            ? tr({ es: "Mirilla", en: "Crosshair" })
            : isSc
              ? tr({ es: "Visor", en: "Scope" })
              : (src.id ?? p.id)),
        ch: isCh
          ? {
              style: src.style ?? "cross",
              size: src.size ?? 48,
              thickness: src.thickness ?? 3,
              gap: src.gap ?? 6,
              color: rgbaToHex(src.color),
              alpha: (src.color?.[3] ?? 240) / 255,
              dot: !!src.dot,
              outline: src.outline ?? true,
            }
          : (prev?.ch ?? defaultCh()),
        sc: isSc
          ? {
              shape: src.shape ?? "circle",
              zoom: src.zoom ?? 2,
              border: src.border ?? true,
              smooth: src.smooth ?? true,
              reticle: src.reticle ?? false,
              aim: src.aim ?? { kind: "under" },
              // An older overlay sends no `activation`; without this default the
              // scope would have no object and the editor would blow up reading
              // it.
              activation: {
                binding: src.activation?.binding ?? null,
                mode: src.activation?.mode ?? "toggle",
                seconds: src.activation?.seconds ?? 3,
              },
            }
          : (prev?.sc ?? defaultSc()),
        x: p.rect.x,
        y: p.rect.y,
        w: p.rect.w,
        h: p.rect.h,
        crop: p.crop ?? { top: 0, right: 0, bottom: 0, left: 0 },
        scale: p.scale ?? "fill",
        z: p.z ?? 0,
        passthrough: p.passthrough ?? true,
        compat: p.compat ?? false,
        passthroughRadius: p.passthrough_radius ?? 90,
        mirror: t?.kind === "all",
        monitorId: t?.kind === "monitor" ? (t.id ?? 0) : (prev?.monitorId ?? 0),
      };
    });
  }

  async function loadMonitors() {
    try {
      const raw = await invoke<string>("screen_list_monitors");
      const parsed = JSON.parse(raw || "[]");
      if (Array.isArray(parsed) && parsed.length) {
        monitors = parsed;
      } else {
        // Platform without native enumeration: a single screen from the web API.
        const m = await primaryMonitor();
        monitors = [
          { id: 0, name: "", x: 0, y: 0, w: m?.size?.width ?? 1920, h: m?.size?.height ?? 1080, primary: true },
        ];
      }
      if (!monitors.find((m) => m.id === activeMonId)) {
        activeMonId = monitors[0]?.id ?? 0;
      }
    } catch (e) {
      error = String(e);
    }
  }

  let unlisten: UnlistenFn | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  function startPolling() {
    stopPolling();
    pollTimer = setInterval(() => {
      loadWindows();
      syncScene();
    }, 2500);
  }
  function stopPolling() {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = null;
  }

  async function openOverlay() {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await loadMonitors();
      await invoke("screen_open", { monitors: monitors.length });
      // Start in Game mode (click-through); the user flips to Edit when ready.
      editing = false;
      await invoke("screen_send", {
        line: JSON.stringify({ type: "set_editor", editor: false }),
      });
      open = true;
      await loadWindows();
      await pushScene();
      startPolling();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function closeOverlay() {
    stopPolling();
    try {
      await invoke("screen_close");
    } catch (e) {
      error = String(e);
    }
    open = false;
    editing = false;
  }

  // The z for a new non-crosshair panel: above the other apps and scopes but below
  // any crosshair (which lives in the +1000 band by default, "always on top"). The
  // Layers list allows reordering afterwards.
  function nextZ(): number {
    return (
      panels
        .filter((p) => p.kind !== "crosshair")
        .reduce((m, p) => Math.max(m, p.z), 0) + 1
    );
  }

  function addPanel(win: WinInfo) {
    const w = Math.round(mon.w / 3);
    const h = Math.round(mon.h / 3);
    const id = `p${Date.now().toString(36)}`;
    const z = nextZ();
    panels.push({
      id,
      kind: "window",
      ch: defaultCh(),
      sc: defaultSc(),
      windowId: win.id,
      label: win.app || win.title || win.id,
      x: Math.round((mon.w - w) / 2),
      y: Math.round((mon.h - h) / 2),
      w,
      h,
      crop: { top: 0, right: 0, bottom: 0, left: 0 },
      scale: "fill",
      z,
      passthrough: true,
      compat: false,
      passthroughRadius: 90,
      monitorId: activeMonId,
      mirror: false,
    });
    selectedId = id;
    note("panel_add", "window");
    pushScene();
  }

  function addCrosshair() {
    const ch = defaultCh();
    const id = `ch${Date.now().toString(36)}`;
    // Banda +1000: la mirilla nace encima de todo (apps y visores).
    const z = panels.reduce((m, p) => Math.max(m, p.z), 0) + 1000;
    panels.push({
      id,
      kind: "crosshair",
      ch,
      sc: defaultSc(),
      windowId: "",
      label: tr({ es: "Mirilla", en: "Crosshair" }),
      x: Math.round((mon.w - ch.size) / 2),
      y: Math.round((mon.h - ch.size) / 2),
      w: ch.size,
      h: ch.size,
      crop: { top: 0, right: 0, bottom: 0, left: 0 },
      scale: "fit",
      z,
      passthrough: true,
      compat: false,
      passthroughRadius: 90,
      monitorId: activeMonId,
      mirror: false,
    });
    selectedId = id;
    note("panel_add", "crosshair");
    pushScene();
  }

  // ---- capturing the scope's binding
  //
  // "Press the button you want to use" and the app detects which one it was. It
  // listens in the capture phase (`capture: true`) and stops propagation: while the
  // binding is being assigned, the click must NOT reach the button underneath or
  // close anything.
  //
  // The mouse is listened to on `pointerdown` rather than `click` because the side
  // buttons (4 and 5) generate no `click` in many browsers, and they are exactly
  // the ones people want to bind. The browser also uses them for back and forward
  // in the history: `preventDefault` stops that.
  let bindingFor = $state<string | null>(null);

  function startBindingCapture(panelId: string) {
    bindingFor = panelId;
  }

  function commitBinding(b: ScBinding | null) {
    const p = panels.find((q) => q.id === bindingFor);
    bindingFor = null;
    if (!p) return;
    p.sc.activation.binding = b;
    // The mode (toggle, hold, timed) says what they want it for; which specific
    // button they picked is nobody's business and does not go up.
    if (b) note("binding", p.sc.activation.mode);
    pushScene();
  }

  function onBindingPointer(e: PointerEvent) {
    if (!bindingFor) return;
    e.preventDefault();
    e.stopPropagation();
    commitBinding({ type: "mouse", button: e.button });
  }

  function onBindingKey(e: KeyboardEvent) {
    if (!bindingFor) return;
    e.preventDefault();
    e.stopPropagation();
    // Escape cancels without binding; it would be the easiest key to bind by
    // accident and the one that makes least sense for a scope.
    if (e.code === "Escape") {
      bindingFor = null;
      return;
    }
    commitBinding({ type: "key", code: e.code });
  }

  $effect(() => {
    if (!bindingFor) return;
    window.addEventListener("pointerdown", onBindingPointer, true);
    window.addEventListener("keydown", onBindingKey, true);
    // `auxclick` is what fires the back/forward menu in some browsers.
    const swallow = (e: Event) => e.preventDefault();
    window.addEventListener("auxclick", swallow, true);
    window.addEventListener("contextmenu", swallow, true);
    return () => {
      window.removeEventListener("pointerdown", onBindingPointer, true);
      window.removeEventListener("keydown", onBindingKey, true);
      window.removeEventListener("auxclick", swallow, true);
      window.removeEventListener("contextmenu", swallow, true);
    };
  });

  function addScope() {
    const size = 360;
    const id = `sc${Date.now().toString(36)}`;
    panels.push({
      id,
      kind: "scope",
      ch: defaultCh(),
      sc: defaultSc(),
      windowId: "",
      label: tr({ es: "Visor", en: "Scope" }),
      x: Math.round((mon.w - size) / 2),
      y: Math.round((mon.h - size) / 2),
      w: size,
      h: size,
      crop: { top: 0, right: 0, bottom: 0, left: 0 },
      scale: "fill",
      z: nextZ(),
      passthrough: true,
      compat: false,
      passthroughRadius: 90,
      monitorId: activeMonId,
      mirror: false,
    });
    selectedId = id;
    note("panel_add", "scope");
    pushScene();
  }

  // Reorders layers by swapping z with the neighbour in the current order, so it
  // crosses the crosshairs' +1000 band cleanly when the user explicitly decides
  // something should go above them.
  function moveLayer(p: Panel, dir: 1 | -1) {
    const order = [...panels].sort((a, b) => a.z - b.z);
    const i = order.indexOf(p);
    const j = i + dir;
    if (j < 0 || j >= order.length) return;
    const other = order[j];
    const tmp = p.z;
    p.z = other.z;
    other.z = tmp;
    if (p.z === other.z) p.z += dir;
    pushScene();
  }

  // Resizes a crosshair keeping its centre (the rect follows the spec so the blit
  // is 1:1 and the edges stay sharp).
  function setChSize(p: Panel, size: number) {
    const cx = p.x + p.w / 2;
    const cy = p.y + p.h / 2;
    p.ch.size = size;
    p.w = size;
    p.h = size;
    p.x = Math.round(cx - size / 2);
    p.y = Math.round(cy - size / 2);
    pushScene();
  }

  function centerPanel(p: Panel) {
    p.x = Math.round((mon.w - p.w) / 2);
    p.y = Math.round((mon.h - p.h) / 2);
    pushScene();
  }

  function setPanelTarget(p: Panel, value: string) {
    if (value === "all") {
      p.mirror = true;
    } else {
      p.mirror = false;
      p.monitorId = Number(value);
    }
    pushScene();
  }

  function removePanel(id: string) {
    const kind = panels.find((p) => p.id === id)?.kind;
    panels = panels.filter((p) => p.id !== id);
    if (selectedId === id) selectedId = null;
    if (kind) note("panel_remove", kind);
    pushScene();
  }


  // --- drag / resize on the preview ---------------------------------------
  const MIN_W = 80;
  const MIN_H = 60;
  const SNAP = 10; // imán en px de preview
  // Un panel puede sobresalir del monitor (la ventana real se coloca off-screen
  // sin problema). Dejamos al menos VIS px dentro para poder volver a agarrarlo
  // en el editor, y permitimos hasta un monitor de desbordamiento por lado.
  const VIS = 48;
  const lo = (size: number, _span: number) => Math.min(VIS - size, 0);
  const hi = (span: number) => span - VIS;

  type Handle = "move" | "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

  // The resize handles: the full-drag one ("move") is the panel's own body; these
  // are the 8 edges and corners painted on selection.
  const HANDLES: { h: Handle; cls: string }[] = [
    { h: "nw", cls: "-left-1 -top-1 cursor-nwse-resize" },
    { h: "n", cls: "left-1/2 -top-1 -translate-x-1/2 cursor-ns-resize" },
    { h: "ne", cls: "-right-1 -top-1 cursor-nesw-resize" },
    { h: "e", cls: "-right-1 top-1/2 -translate-y-1/2 cursor-ew-resize" },
    { h: "se", cls: "-right-1 -bottom-1 cursor-nwse-resize" },
    { h: "s", cls: "left-1/2 -bottom-1 -translate-x-1/2 cursor-ns-resize" },
    { h: "sw", cls: "-left-1 -bottom-1 cursor-nesw-resize" },
    { h: "w", cls: "-left-1 top-1/2 -translate-y-1/2 cursor-ew-resize" },
  ];

  const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));

  // The picker's crosshair styles (typed here to avoid casting in the template).
  const CH_STYLES: { st: Ch["style"]; glyph: string }[] = [
    { st: "cross", glyph: "+" },
    { st: "x", glyph: "×" },
    { st: "dot", glyph: "•" },
    { st: "circle", glyph: "○" },
  ];

  // Imanta un valor a uno de varios objetivos si queda dentro del umbral
  // (expresado en px de monitor, por eso dividimos SNAP por la escala).
  function snap(v: number, targets: number[]): number {
    const tol = SNAP / previewScale;
    for (const t of targets) if (Math.abs(v - t) <= tol) return t;
    return v;
  }

  let drag: {
    id: string;
    handle: Handle;
    sx: number;
    sy: number;
    ox: number;
    oy: number;
    ow: number;
    oh: number;
  } | null = null;

  function onPointerDown(e: PointerEvent, p: Panel, handle: Handle) {
    e.preventDefault();
    e.stopPropagation();
    selectedId = p.id;
    drag = { id: p.id, handle, sx: e.clientX, sy: e.clientY, ox: p.x, oy: p.y, ow: p.w, oh: p.h };
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  }

  function onPointerMove(e: PointerEvent) {
    if (!drag) return;
    const p = panels.find((q) => q.id === drag!.id);
    if (!p) return;
    const dx = (e.clientX - drag.sx) / previewScale;
    const dy = (e.clientY - drag.sy) / previewScale;
    const free = e.shiftKey; // Shift desactiva el imán

    if (drag.handle === "move") {
      let nx = drag.ox + dx;
      let ny = drag.oy + dy;
      if (!free) {
        nx = snap(nx, [0, (mon.w - p.w) / 2, mon.w - p.w]);
        ny = snap(ny, [0, (mon.h - p.h) / 2, mon.h - p.h]);
      }
      p.x = clamp(Math.round(nx), lo(p.w, mon.w), hi(mon.w));
      p.y = clamp(Math.round(ny), lo(p.h, mon.h), hi(mon.h));
    } else {
      const h = drag.handle;
      let left = drag.ox;
      let top = drag.oy;
      let right = drag.ox + drag.ow;
      let bottom = drag.oy + drag.oh;
      if (h.includes("e")) right = drag.ox + drag.ow + dx;
      if (h.includes("w")) left = drag.ox + dx;
      if (h.includes("s")) bottom = drag.oy + drag.oh + dy;
      if (h.includes("n")) top = drag.oy + dy;
      if (!free) {
        if (h.includes("e")) right = snap(right, [mon.w, mon.w / 2]);
        if (h.includes("w")) left = snap(left, [0, mon.w / 2]);
        if (h.includes("s")) bottom = snap(bottom, [mon.h, mon.h / 2]);
        if (h.includes("n")) top = snap(top, [0, mon.h / 2]);
      }
      // Permite que los bordes salgan del monitor (hasta un monitor por lado).
      left = clamp(left, -mon.w, 2 * mon.w);
      right = clamp(right, -mon.w, 2 * mon.w);
      top = clamp(top, -mon.h, 2 * mon.h);
      bottom = clamp(bottom, -mon.h, 2 * mon.h);
      // Honours the minimum size by moving only the edge being dragged.
      if (h.includes("w")) left = Math.min(left, right - MIN_W);
      else right = Math.max(right, left + MIN_W);
      if (h.includes("n")) top = Math.min(top, bottom - MIN_H);
      else bottom = Math.max(bottom, top + MIN_H);
      p.x = Math.round(left);
      p.y = Math.round(top);
      p.w = Math.round(right - left);
      p.h = Math.round(bottom - top);
    }
    pushScene();
  }

  function onPointerUp() {
    drag = null;
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
  }

  // Mover/redimensionar el panel seleccionado con el teclado (flechas; Shift =
  // paso de 10; Alt = redimensiona en vez de mover).
  function onKeydown(e: KeyboardEvent) {
    const s = selected;
    if (!s || cropMode) return;
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
    if (!e.key.startsWith("Arrow")) return;
    const step = e.shiftKey ? 10 : 1;
    if (e.altKey) {
      // A crosshair's size is changed with its slider (rect = spec, 1:1).
      if (s.kind === "crosshair") return;
      if (e.key === "ArrowRight") s.w = Math.max(MIN_W, s.w + step);
      else if (e.key === "ArrowLeft") s.w = Math.max(MIN_W, s.w - step);
      else if (e.key === "ArrowDown") s.h = Math.max(MIN_H, s.h + step);
      else s.h = Math.max(MIN_H, s.h - step);
    } else {
      if (e.key === "ArrowRight") s.x = clamp(s.x + step, lo(s.w, mon.w), hi(mon.w));
      else if (e.key === "ArrowLeft") s.x = clamp(s.x - step, lo(s.w, mon.w), hi(mon.w));
      else if (e.key === "ArrowDown") s.y = clamp(s.y + step, lo(s.h, mon.h), hi(mon.h));
      else s.y = clamp(s.y - step, lo(s.h, mon.h), hi(mon.h));
    }
    e.preventDefault();
    pushScene();
  }

  // Direct numeric editing of the panel's box.
  function setRect(p: Panel, key: "x" | "y" | "w" | "h", v: number) {
    if (!Number.isFinite(v)) return;
    v = Math.round(v);
    if (key === "w") p.w = Math.max(MIN_W, v);
    else if (key === "h") p.h = Math.max(MIN_H, v);
    else if (key === "x") p.x = clamp(v, lo(p.w, mon.w), hi(mon.w));
    else p.y = clamp(v, lo(p.h, mon.h), hi(mon.h));
    pushScene();
  }

  // ---- crop
  // The selected panel's visual crop mode: the whole window is visible and whatever
  // falls outside the crop is dimmed, with draggable handles on each edge.
  let cropId = $state<string | null>(null);
  const cropMode = $derived(cropId === selectedId && !!selected);
  function toggleCrop() {
    cropId = cropMode ? null : selectedId;
  }

  function setCrop(p: Panel, edge: keyof Panel["crop"], v: number) {
    // El recorte no puede tragarse el borde opuesto: deja al menos un 10% vivo.
    const opp =
      edge === "top" ? "bottom" : edge === "bottom" ? "top" : edge === "left" ? "right" : "left";
    const max = Math.max(0, 0.9 - p.crop[opp]);
    p.crop[edge] = clamp(v, 0, max);
    pushScene();
  }

  function resetCrop(p: Panel) {
    p.crop = { top: 0, right: 0, bottom: 0, left: 0 };
    pushScene();
  }

  let cropDrag: {
    id: string;
    edge: keyof Panel["crop"];
    sx: number;
    sy: number;
    start: number;
  } | null = null;

  function onCropDown(e: PointerEvent, p: Panel, edge: keyof Panel["crop"]) {
    e.preventDefault();
    e.stopPropagation();
    cropDrag = { id: p.id, edge, sx: e.clientX, sy: e.clientY, start: p.crop[edge] };
    window.addEventListener("pointermove", onCropMove);
    window.addEventListener("pointerup", onCropUp);
  }

  function onCropMove(e: PointerEvent) {
    if (!cropDrag) return;
    const p = panels.find((q) => q.id === cropDrag!.id);
    if (!p) return;
    const pxW = p.w * previewScale;
    const pxH = p.h * previewScale;
    const d = cropDrag;
    let v = d.start;
    if (d.edge === "left") v = d.start + (e.clientX - d.sx) / pxW;
    else if (d.edge === "right") v = d.start - (e.clientX - d.sx) / pxW;
    else if (d.edge === "top") v = d.start + (e.clientY - d.sy) / pxH;
    else v = d.start - (e.clientY - d.sy) / pxH;
    setCrop(p, d.edge, v);
  }

  function onCropUp() {
    cropDrag = null;
    window.removeEventListener("pointermove", onCropMove);
    window.removeEventListener("pointerup", onCropUp);
  }

  const pct = (v: number) => `${Math.round(v * 100)}%`;

  onMount(async () => {
    window.addEventListener("keydown", onKeydown);
    try {
      unlisten = await listen<string>("screen://event", (ev) => {
        try {
          const msg = JSON.parse(ev.payload);
          if (msg.type === "mode") editing = !!msg.editor;
          else if (msg.type === "scene") applyIncomingScene(msg.scene);
        } catch {
          /* ignore malformed */
        }
      });
    } catch {
      /* no event bus (shouldn't happen) */
    }
    try {
      open = await invoke<boolean>("screen_is_open");
      if (open) {
        // The overlay is still alive from an earlier mount: paint the local buffer
        // at once and ask the overlay for the REAL scene (the local copy can be
        // empty or stale after a reload, the undeletable-panel bug).
        panels = savedPanels;
        await loadMonitors();
        await loadWindows();
        startPolling();
        syncScene();
      }
    } catch {
      /* community build: command missing → stays closed */
    }
  });

  onDestroy(() => {
    // Preserves the live scene for the next mount (the overlay does not close when
    // the view changes, only this control panel unmounts).
    savedPanels = panels;
    stopPolling();
    unlisten?.();
    window.removeEventListener("keydown", onKeydown);
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointermove", onCropMove);
    window.removeEventListener("pointerup", onCropUp);
  });
</script>

<div class="mx-auto max-w-5xl px-6 py-8">
  <div class="mb-6 flex items-center gap-3">
    <div
      class="grid h-12 w-12 place-items-center rounded-xl bg-emerald-500/10 ring-1 ring-emerald-500/30"
    >
      <MonitorPlay size={26} class="text-emerald-400" />
    </div>
    <div>
      <h1 class="text-xl font-semibold text-zinc-50">Hoard Screen</h1>
      <p class="text-sm text-zinc-400">
        {tr({
          es: "Una capa nativa sobre el juego: captura ventanas de otras apps y colócalas flotando encima. Edítalo desde aquí; el overlay refleja los cambios en vivo.",
          en: "A native layer over your game: capture other apps' windows and float them on top. Edit here; the overlay reflects changes live.",
        })}
      </p>
    </div>
  </div>

  {#if error}
    <div
      class="mb-4 rounded-lg border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-200"
    >
      {error}
    </div>
  {/if}

  {#if !open}
    <button
      type="button"
      onclick={openOverlay}
      disabled={busy}
      class="flex items-center gap-2 rounded-lg bg-emerald-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-60"
    >
      <SquarePlus size={18} />
      {tr({ es: "Abrir overlay", en: "Open overlay" })}
    </button>
  {:else}
    <div class="grid gap-6 md:grid-cols-[1fr_18rem]">
      <!-- preview -->
      <div bind:clientWidth={previewColWidth}>
        <div class="mb-2 flex flex-wrap items-center gap-2">
          <button
            type="button"
            onclick={closeOverlay}
            class="flex items-center gap-2 rounded-lg border border-zinc-700 px-3 py-1.5 text-sm text-zinc-300 hover:bg-zinc-800"
          >
            <SquareX size={15} />
            {tr({ es: "Ocultar overlay", en: "Hide overlay" })}
          </button>
          <div class="inline-flex rounded-md border border-zinc-700 p-0.5">
            <button
              type="button"
              onclick={() => setEditor(false)}
              class="flex items-center gap-1 rounded px-2.5 py-1 text-xs {!editing
                ? 'bg-emerald-600 text-white'
                : 'text-zinc-300'}"
              ><Gamepad2 size={14} /> {tr({ es: "Juego", en: "Game" })}</button
            >
            <button
              type="button"
              onclick={() => setEditor(true)}
              class="flex items-center gap-1 rounded px-2.5 py-1 text-xs {editing
                ? 'bg-emerald-600 text-white'
                : 'text-zinc-300'}"
              ><Pencil size={14} /> {tr({ es: "Editar", en: "Edit" })}</button
            >
          </div>
          <span class="text-xs text-zinc-500">{mon.w}×{mon.h}</span>
        </div>
        <p class="mb-2 flex items-start gap-1 text-[11px] text-zinc-500">
          <TriangleAlert size={12} class="mt-0.5 shrink-0 text-zinc-600" />
          <span
            >{tr({
              es: "El juego debe ir en ventana sin bordes (borderless), no en pantalla completa exclusiva, o no se verá nada encima.",
              en: "Run the game in borderless windowed mode, not exclusive fullscreen, or nothing will show on top.",
            })}</span
          >
        </p>
        {#if monitors.length > 1}
          <div class="mb-2 flex flex-wrap items-center gap-1">
            {#each monitors as m (m.id)}
              <button
                type="button"
                onclick={() => (activeMonId = m.id)}
                class="rounded-md border px-2.5 py-1 text-xs {activeMonId === m.id
                  ? 'border-emerald-500/60 bg-emerald-600/20 text-emerald-200'
                  : 'border-zinc-700 text-zinc-300 hover:bg-zinc-800'}"
                title="{m.w}×{m.h}">{monLabel(m)}</button
              >
            {/each}
            <span class="ml-1 text-[11px] text-zinc-500"
              >{tr({
                es: "Arrastra cada app en la pantalla elegida; usa el selector del panel para moverla a otra o ponerla en espejo.",
                en: "Arrange each app on the chosen screen; use the panel selector to move it to another or mirror it.",
              })}</span
            >
          </div>
        {/if}
        <p class="mb-2 text-[11px] text-zinc-500">
          {tr({
            es: "Modo Editar: cada app se vuelve una ventana normal que mueves y redimensionas por cualquier borde. Al volver a Juego, la captura queda donde la dejaste. Ctrl+O o Esc cambian de modo.",
            en: "Edit mode: each app becomes a normal window you move and resize from any edge. Back in Game, the capture stays where you left it. Ctrl+O or Esc switch modes.",
          })}
        </p>
        <div
          class="relative overflow-hidden rounded-lg border border-zinc-700 bg-zinc-900"
          style="width:{PREVIEW_W}px;height:{previewH}px"
          role="presentation"
          onpointerdown={() => (selectedId = null)}
        >
          {#each visiblePanels as p (p.id)}
            {@const isSel = selectedId === p.id}
            {@const inCrop = isSel && cropMode}
            <div
              role="button"
              tabindex="0"
              class="absolute select-none rounded-sm border text-[10px] {isSel
                ? 'border-emerald-400 bg-emerald-500/20'
                : 'border-zinc-500/70 bg-zinc-700/40'} {inCrop ? 'cursor-default' : 'cursor-move'}"
              style="left:{p.x * previewScale}px;top:{p.y *
                previewScale}px;width:{p.w * previewScale}px;height:{p.h *
                previewScale}px;z-index:{p.z}"
              onpointerdown={(e) => !inCrop && onPointerDown(e, p, "move")}
            >
              {#if p.kind === "crosshair"}
                <span
                  class="pointer-events-none absolute inset-0 grid select-none place-items-center leading-none"
                  style="color:{p.ch.color};opacity:{p.ch.alpha};font-size:{Math.max(
                    10,
                    p.w * previewScale,
                  )}px"
                  >{p.ch.style === "x"
                    ? "×"
                    : p.ch.style === "dot"
                      ? "•"
                      : p.ch.style === "circle"
                        ? "○"
                        : "+"}</span
                >
              {:else if p.kind === "scope"}
                <span
                  class="pointer-events-none absolute inset-1 grid select-none place-items-center border-2 border-zinc-300/70 text-[10px] text-zinc-200 {p.sc
                    .shape === 'circle'
                    ? 'rounded-full'
                    : 'rounded-sm'}">×{p.sc.zoom}</span
                >
              {:else}
                <span
                  class="pointer-events-none absolute inset-x-0 top-0 truncate px-1 py-0.5 text-zinc-100"
                  >{p.label}</span
                >
              {/if}

              {#if inCrop}
                <!-- Capa de recorte: lo que cae fuera se atenúa; los bordes del
                     área retenida se arrastran para recortar. -->
                <div
                  class="pointer-events-none absolute inset-x-0 top-0 bg-zinc-950/70"
                  style="height:{pct(p.crop.top)}"
                ></div>
                <div
                  class="pointer-events-none absolute inset-x-0 bottom-0 bg-zinc-950/70"
                  style="height:{pct(p.crop.bottom)}"
                ></div>
                <div
                  class="pointer-events-none absolute left-0 bg-zinc-950/70"
                  style="top:{pct(p.crop.top)};bottom:{pct(p.crop.bottom)};width:{pct(p.crop.left)}"
                ></div>
                <div
                  class="pointer-events-none absolute right-0 bg-zinc-950/70"
                  style="top:{pct(p.crop.top)};bottom:{pct(p.crop.bottom)};width:{pct(p.crop.right)}"
                ></div>
                <div
                  class="pointer-events-none absolute border border-dashed border-emerald-300"
                  style="left:{pct(p.crop.left)};right:{pct(p.crop.right)};top:{pct(
                    p.crop.top,
                  )};bottom:{pct(p.crop.bottom)}"
                ></div>
                <div
                  role="button"
                  tabindex="0"
                  class="absolute z-10 h-1.5 w-6 -translate-x-1/2 -translate-y-1/2 cursor-ns-resize rounded-full bg-emerald-400"
                  style="left:{pct((p.crop.left + 1 - p.crop.right) / 2)};top:{pct(p.crop.top)}"
                  onpointerdown={(e) => onCropDown(e, p, "top")}
                ></div>
                <div
                  role="button"
                  tabindex="0"
                  class="absolute z-10 h-1.5 w-6 -translate-x-1/2 translate-y-1/2 cursor-ns-resize rounded-full bg-emerald-400"
                  style="left:{pct((p.crop.left + 1 - p.crop.right) / 2)};bottom:{pct(p.crop.bottom)}"
                  onpointerdown={(e) => onCropDown(e, p, "bottom")}
                ></div>
                <div
                  role="button"
                  tabindex="0"
                  class="absolute z-10 h-6 w-1.5 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize rounded-full bg-emerald-400"
                  style="top:{pct((p.crop.top + 1 - p.crop.bottom) / 2)};left:{pct(p.crop.left)}"
                  onpointerdown={(e) => onCropDown(e, p, "left")}
                ></div>
                <div
                  role="button"
                  tabindex="0"
                  class="absolute z-10 h-6 w-1.5 translate-x-1/2 -translate-y-1/2 cursor-ew-resize rounded-full bg-emerald-400"
                  style="top:{pct((p.crop.top + 1 - p.crop.bottom) / 2)};right:{pct(p.crop.right)}"
                  onpointerdown={(e) => onCropDown(e, p, "right")}
                ></div>
              {:else if isSel && p.kind !== "crosshair"}
                <!-- tiradores de redimensión (una mirilla se dimensiona con su
                     slider de tamaño; solo se arrastra) -->
                {#each HANDLES as hd (hd.h)}
                  <div
                    role="button"
                    tabindex="0"
                    class="absolute z-10 h-2 w-2 rounded-sm border border-emerald-200 bg-emerald-400 {hd.cls}"
                    onpointerdown={(e) => onPointerDown(e, p, hd.h)}
                  ></div>
                {/each}
              {/if}
            </div>
          {/each}
        </div>

        {#if selected}
          {@const s = selected}
          <div class="mt-3 space-y-3 rounded-lg border border-zinc-700 bg-zinc-800/40 p-3">
            <div class="flex items-center justify-between">
              <span class="truncate text-sm font-medium text-zinc-100">{s.label}</span>
              <div class="flex items-center gap-1">
                <button
                  type="button"
                  title={tr({ es: "Subir capa", en: "Raise layer" })}
                  onclick={() => moveLayer(s, 1)}
                  class="rounded p-1 text-zinc-300 hover:bg-zinc-700"><ArrowUp size={14} /></button
                >
                <button
                  type="button"
                  title={tr({ es: "Bajar capa", en: "Lower layer" })}
                  onclick={() => moveLayer(s, -1)}
                  class="rounded p-1 text-zinc-300 hover:bg-zinc-700"
                  ><ArrowDown size={14} /></button
                >
                <button
                  type="button"
                  onclick={() => removePanel(s.id)}
                  class="rounded p-1 text-red-300 hover:bg-red-500/20"><Trash2 size={14} /></button
                >
              </div>
            </div>
            <div class="grid grid-cols-4 gap-1.5">
              {#each s.kind === "crosshair" ? [["x", "X"], ["y", "Y"]] : [["x", "X"], ["y", "Y"], ["w", tr({ es: "An", en: "W" })], ["h", tr({ es: "Al", en: "H" })]] as [key, lbl]}
                <label class="flex flex-col gap-0.5 text-[10px] text-zinc-500">
                  <span>{lbl}</span>
                  <input
                    type="number"
                    value={s[key as "x" | "y" | "w" | "h"]}
                    onchange={(e) =>
                      setRect(s, key as "x" | "y" | "w" | "h", +e.currentTarget.value)}
                    class="w-full rounded border border-zinc-700 bg-zinc-900 px-1.5 py-1 text-xs text-zinc-200"
                  />
                </label>
              {/each}
            </div>
            {#if monitors.length > 1}
              <label class="flex items-center gap-2 text-xs text-zinc-400">
                <span>{tr({ es: "Pantalla", en: "Screen" })}</span>
                <select
                  value={s.mirror ? "all" : String(s.monitorId)}
                  onchange={(e) => setPanelTarget(s, e.currentTarget.value)}
                  class="flex-1 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-zinc-200"
                >
                  {#each monitors as m (m.id)}
                    <option value={String(m.id)}>{monLabel(m)} ({m.w}×{m.h})</option>
                  {/each}
                  <option value="all">{tr({ es: "Espejo (todas)", en: "Mirror (all)" })}</option>
                </select>
              </label>
            {/if}
            {#if s.kind === "window"}
            <div class="inline-flex rounded-md border border-zinc-700 p-0.5">
              <button
                type="button"
                onclick={() => {
                  s.scale = "fit";
                  pushScene();
                }}
                class="flex items-center gap-1 rounded px-2 py-1 text-xs {s.scale === 'fit'
                  ? 'bg-emerald-600 text-white'
                  : 'text-zinc-300'}"><Proportions size={13} /> {tr({ es: "Ajustar", en: "Fit" })}</button
              >
              <button
                type="button"
                onclick={() => {
                  s.scale = "fill";
                  pushScene();
                }}
                class="flex items-center gap-1 rounded px-2 py-1 text-xs {s.scale === 'fill'
                  ? 'bg-emerald-600 text-white'
                  : 'text-zinc-300'}"><Maximize size={13} /> {tr({ es: "Llenar", en: "Fill" })}</button
              >
            </div>
            <div class="flex items-center justify-between">
              <button
                type="button"
                onclick={toggleCrop}
                class="flex items-center gap-1 rounded px-2 py-1 text-xs {cropMode
                  ? 'bg-emerald-600 text-white'
                  : 'border border-zinc-700 text-zinc-300 hover:bg-zinc-700'}"
                ><Crop size={13} />
                {tr({ es: "Recortar", en: "Crop" })}</button
              >
              <button
                type="button"
                title={tr({ es: "Quitar recorte", en: "Reset crop" })}
                onclick={() => resetCrop(s)}
                class="rounded p-1 text-zinc-400 hover:bg-zinc-700 hover:text-white"
                ><RotateCcw size={13} /></button
              >
            </div>
            <div class="grid grid-cols-2 gap-2 text-xs text-zinc-400">
              {#each [["top", "↑"], ["bottom", "↓"], ["left", "←"], ["right", "→"]] as [edge, sym]}
                <label class="flex items-center gap-2">
                  <span class="w-3">{sym}</span>
                  <input
                    type="range"
                    min="0"
                    max="0.9"
                    step="0.01"
                    value={s.crop[edge as keyof Panel["crop"]]}
                    oninput={(e) =>
                      setCrop(s, edge as keyof Panel["crop"], +e.currentTarget.value)}
                    class="flex-1"
                  />
                  <span class="w-8 text-right tabular-nums text-zinc-500"
                    >{pct(s.crop[edge as keyof Panel["crop"]])}</span
                  >
                </label>
              {/each}
            </div>
            <p class="text-[11px] text-zinc-500">
              {cropMode
                ? tr({
                    es: "Arrastra los bordes en la vista previa; lo atenuado se recorta.",
                    en: "Drag the edges in the preview; the dimmed area is cropped.",
                  })
                : tr({ es: "Recorte (fracción del origen)", en: "Crop (fraction of source)" })}
            </p>
            <div class="mt-1 border-t border-zinc-800 pt-2">
              <button
                type="button"
                onclick={() => {
                  s.passthrough = !s.passthrough;
                  pushScene();
                }}
                class="flex w-full items-center justify-between gap-1 rounded px-2 py-1 text-xs {s.passthrough
                  ? 'text-zinc-300'
                  : 'bg-amber-600/20 text-amber-300'}"
              >
                <span class="flex items-center gap-1"
                  ><MousePointerClick size={13} />
                  {tr({ es: "Clics pasan al juego", en: "Clicks go to game" })}</span
                >
                <span class="tabular-nums">{s.passthrough ? "ON" : "OFF"}</span>
              </button>
              <p class="px-2 text-[11px] text-zinc-500">
                {tr({
                  es: "Desactívalo solo si este vídeo sale en negro (Prime/Netflix). Entonces los clics sobre este panel irán a la app, no al juego.",
                  en: "Turn off only if this video shows black (Prime/Netflix). Then clicks on this panel go to the app, not the game.",
                })}
              </p>
              {#if s.passthrough}
                <div class="mt-1 flex items-center gap-2 px-2">
                  <span class="text-xs text-zinc-300">
                    {tr({ es: "Lente click-through", en: "Click-through lens" })}
                  </span>
                  <input
                    type="range"
                    min="20"
                    max="400"
                    step="5"
                    bind:value={s.passthroughRadius}
                    oninput={() => pushScene()}
                    class="flex-1 accent-zinc-400"
                  />
                  <span class="w-10 text-right text-xs tabular-nums text-zinc-400"
                    >{s.passthroughRadius}px</span
                  >
                </div>
                <p class="px-2 text-[11px] text-zinc-500">
                  {tr({
                    es: "Círculo que sigue al cursor: dentro ves y clicas lo que hay detrás del panel.",
                    en: "Circle that follows the cursor: inside it you see and click whatever is behind the panel.",
                  })}
                </p>
              {/if}
            </div>
            <div class="mt-1 border-t border-zinc-800 pt-2">
              <button
                type="button"
                onclick={() => {
                  s.compat = !s.compat;
                  pushScene();
                }}
                class="flex w-full items-center justify-between gap-1 rounded px-2 py-1 text-xs {s.compat
                  ? 'bg-emerald-600/20 text-emerald-300'
                  : 'text-zinc-300'}"
              >
                <span class="flex items-center gap-1"
                  ><Crop size={13} />
                  {tr({ es: "Modo compatibilidad Chromium", en: "Chromium compatibility mode" })}</span
                >
                <span class="tabular-nums">{s.compat ? "ON" : "OFF"}</span>
              </button>
              <p class="px-2 text-[11px] text-zinc-500">
                {tr({
                  es: "Recorte limpio en Brave/Discord (Chromium) y permite encoger el panel por debajo del mínimo de la ventana. Para clicar el panel (pausar, etc.), apaga «Clics pasan al juego» en él: los clics se reenvían a la ventana. Es posible que este modo cause que la ventana se quede en negro al reproducir.",
                  en: "Clean crop on Brave/Discord (Chromium), and lets the panel shrink below the window's minimum size. To click the panel (pause, etc.), turn off \"Clicks pass to game\" on it: clicks are forwarded to the window. This mode may make the window go black while playing video.",
                })}
              </p>
            </div>
            {:else if s.kind === "scope"}
              <!-- controles del visor -->
              <div class="flex items-center gap-2">
                <div class="inline-flex rounded-md border border-zinc-700 p-0.5">
                  <button
                    type="button"
                    onclick={() => {
                      s.sc.shape = "circle";
                      pushScene();
                    }}
                    class="w-7 rounded py-1 text-sm leading-none {s.sc.shape === 'circle'
                      ? 'bg-emerald-600 text-white'
                      : 'text-zinc-300 hover:bg-zinc-700'}">○</button
                  >
                  <button
                    type="button"
                    onclick={() => {
                      s.sc.shape = "square";
                      pushScene();
                    }}
                    class="w-7 rounded py-1 text-sm leading-none {s.sc.shape === 'square'
                      ? 'bg-emerald-600 text-white'
                      : 'text-zinc-300 hover:bg-zinc-700'}">□</button
                  >
                </div>
                <button
                  type="button"
                  title={tr({ es: "Centrar en la pantalla", en: "Center on screen" })}
                  onclick={() => centerPanel(s)}
                  class="rounded border border-zinc-700 p-1.5 text-zinc-300 hover:bg-zinc-700"
                  ><Locate size={14} /></button
                >
                <button
                  type="button"
                  onclick={() => {
                    s.sc.border = !s.sc.border;
                    pushScene();
                  }}
                  class="flex items-center justify-between gap-2 rounded border border-zinc-700 px-2 py-1 text-xs {s.sc
                    .border
                    ? 'bg-emerald-600/20 text-emerald-300'
                    : 'text-zinc-300 hover:bg-zinc-700'}"
                >
                  <span>{tr({ es: "Borde", en: "Border" })}</span>
                  <span class="tabular-nums">{s.sc.border ? "ON" : "OFF"}</span>
                </button>
              </div>
              <label class="flex items-center gap-2 text-xs text-zinc-400">
                <span class="w-16 shrink-0">{tr({ es: "Aumento", en: "Zoom" })}</span>
                <input
                  type="range"
                  min="0"
                  max={ZOOM_STEPS}
                  step="1"
                  value={zoomToSlider(s.sc.zoom)}
                  oninput={(e) => {
                    s.sc.zoom = sliderToZoom(+e.currentTarget.value);
                    pushScene();
                  }}
                  class="flex-1 accent-emerald-500"
                  aria-label={tr({ es: "Aumento", en: "Zoom" })}
                  aria-valuetext={zoomLabel(s.sc.zoom)}
                />
                <!-- Campo numérico además de la barra: con un rango tan amplio,
                     clavar un ×1,01 o un ×12,5 arrastrando es una lotería. -->
                <input
                  type="number"
                  min={ZOOM_MIN}
                  max={ZOOM_MAX}
                  step="0.01"
                  value={s.sc.zoom}
                  oninput={(e) => {
                    const v = +e.currentTarget.value;
                    if (!Number.isFinite(v)) return;
                    s.sc.zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, v));
                    pushScene();
                  }}
                  class="w-16 shrink-0 rounded border border-zinc-700 bg-zinc-900 px-1.5 py-1 text-right text-xs tabular-nums text-zinc-200 [appearance:textfield] focus:border-emerald-500/40 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
                />
                <span class="w-12 shrink-0 text-right tabular-nums text-zinc-500"
                  >{zoomLabel(s.sc.zoom)}</span
                >
              </label>

              <!-- ── Nitidez y retícula ────────────────────────────────── -->
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  title={tr({
                    es: "Con aumento alto, «Suave» difumina y «Nítido» deja los píxeles duros",
                    en: "At high zoom, Smooth blurs and Sharp keeps hard pixel edges",
                  })}
                  onclick={() => {
                    s.sc.smooth = !s.sc.smooth;
                    pushScene();
                  }}
                  class="flex-1 rounded border border-zinc-700 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-700"
                >
                  {s.sc.smooth
                    ? tr({ es: "Suave", en: "Smooth" })
                    : tr({ es: "Nítido", en: "Sharp" })}
                </button>
                <button
                  type="button"
                  onclick={() => {
                    s.sc.reticle = !s.sc.reticle;
                    pushScene();
                  }}
                  class="flex-1 rounded border px-2 py-1 text-xs transition-colors {s.sc.reticle
                    ? 'border-emerald-500/60 bg-emerald-600/20 text-emerald-300'
                    : 'border-zinc-700 text-zinc-300 hover:bg-zinc-700'}"
                >
                  {tr({ es: "Retícula", en: "Reticle" })}
                  <span class="tabular-nums">{s.sc.reticle ? "ON" : "OFF"}</span>
                </button>
              </div>

              <!-- ── A qué apunta ──────────────────────────────────────── -->
              <div class="flex items-center gap-2 text-xs text-zinc-400">
                <span class="w-16 shrink-0">{tr({ es: "Apunta a", en: "Aims at" })}</span>
                <div class="inline-flex flex-1 rounded-md border border-zinc-700 p-0.5">
                  {#each [{ k: "under", es: "Debajo", en: "Under" }, { k: "center", es: "Centro", en: "Center" }, { k: "offset", es: "Desplazado", en: "Offset" }] as opt (opt.k)}
                    <button
                      type="button"
                      onclick={() => {
                        s.sc.aim =
                          opt.k === "offset"
                            ? { kind: "offset", dx: 0, dy: -200 }
                            : { kind: opt.k as "under" | "center" };
                        pushScene();
                      }}
                      class="flex-1 rounded py-1 text-[11px] leading-none transition-colors {s.sc.aim
                        .kind === opt.k
                        ? 'bg-emerald-600 text-white'
                        : 'text-zinc-300 hover:bg-zinc-700'}"
                      >{tr({ es: opt.es, en: opt.en })}</button
                    >
                  {/each}
                </div>
              </div>

              {#if s.sc.aim.kind === "offset"}
                <div class="flex items-center gap-2 text-xs text-zinc-400">
                  <span class="w-16 shrink-0">{tr({ es: "Distancia", en: "Distance" })}</span>
                  {#each [{ ax: "dx" as const, lbl: "X" }, { ax: "dy" as const, lbl: "Y" }] as f (f.ax)}
                    <label class="flex flex-1 items-center gap-1">
                      <span class="text-zinc-500">{f.lbl}</span>
                      <input
                        type="number"
                        step="10"
                        value={s.sc.aim.kind === "offset" ? s.sc.aim[f.ax] : 0}
                        oninput={(e) => {
                          const v = +e.currentTarget.value;
                          if (s.sc.aim.kind !== "offset" || !Number.isFinite(v)) return;
                          s.sc.aim = { ...s.sc.aim, [f.ax]: v };
                          pushScene();
                        }}
                        class="w-full rounded border border-zinc-700 bg-zinc-900 px-1.5 py-1 text-right tabular-nums text-zinc-200 [appearance:textfield] focus:border-emerald-500/40 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
                      />
                    </label>
                  {/each}
                </div>
              {/if}

              <p class="text-[11px] leading-relaxed text-zinc-500">
                {s.sc.aim.kind === "under"
                  ? tr({
                      es: "Amplía lo que tiene justo debajo. Para ver el centro de la pantalla hay que ponerla encima… y entonces lo tapa.",
                      en: "Magnifies whatever sits under it. To see the screen centre you have to put it there — and then it covers it.",
                    })
                  : s.sc.aim.kind === "center"
                    ? tr({
                        es: "Amplía el centro de la pantalla estés donde estés: deja la lente en una esquina y sigue viendo el punto de mira.",
                        en: "Magnifies the screen centre wherever the lens sits: park it in a corner and still watch your crosshair.",
                      })
                    : tr({
                        es: "Amplía un punto desplazado respecto a la lente. Y negativa sube; X positiva va a la derecha.",
                        en: "Magnifies a point offset from the lens. Negative Y is up; positive X is right.",
                      })}
              </p>

              <!-- ── Activación ────────────────────────────────────────── -->
              <div class="mt-1 space-y-2 border-t border-zinc-700/60 pt-2">
                <div class="flex items-center gap-2 text-xs text-zinc-400">
                  <span class="w-16 shrink-0">{tr({ es: "Botón", en: "Button" })}</span>
                  <button
                    type="button"
                    onclick={() => startBindingCapture(s.id)}
                    class="flex-1 rounded border px-2 py-1 text-left text-xs transition-colors {bindingFor ===
                    s.id
                      ? 'animate-pulse border-emerald-500 bg-emerald-600/20 text-emerald-200'
                      : 'border-zinc-700 text-zinc-200 hover:bg-zinc-700'}"
                  >
                    {bindingFor === s.id
                      ? tr({
                          es: "Pulsa un botón o tecla… (Esc cancela)",
                          en: "Press a button or key… (Esc cancels)",
                        })
                      : bindingLabel(s.sc.activation.binding)}
                  </button>
                  {#if s.sc.activation.binding && bindingFor !== s.id}
                    <button
                      type="button"
                      title={tr({ es: "Quitar el vínculo", en: "Clear binding" })}
                      onclick={() => {
                        s.sc.activation.binding = null;
                        pushScene();
                      }}
                      class="shrink-0 rounded border border-zinc-700 px-2 py-1 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
                      >×</button
                    >
                  {/if}
                </div>

                {#if s.sc.activation.binding}
                  <div class="flex items-center gap-2 text-xs text-zinc-400">
                    <span class="w-16 shrink-0">{tr({ es: "Modo", en: "Mode" })}</span>
                    <div class="inline-flex flex-1 rounded-md border border-zinc-700 p-0.5">
                      {#each [{ m: "toggle" as ScMode, es: "Alternar", en: "Toggle" }, { m: "hold" as ScMode, es: "Mantener", en: "Hold" }, { m: "timed" as ScMode, es: "Segundos", en: "Timed" }] as opt (opt.m)}
                        <button
                          type="button"
                          onclick={() => {
                            s.sc.activation.mode = opt.m;
                            pushScene();
                          }}
                          class="flex-1 rounded py-1 text-[11px] leading-none transition-colors {s.sc
                            .activation.mode === opt.m
                            ? 'bg-emerald-600 text-white'
                            : 'text-zinc-300 hover:bg-zinc-700'}"
                          >{tr({ es: opt.es, en: opt.en })}</button
                        >
                      {/each}
                    </div>
                  </div>

                  {#if s.sc.activation.mode === "timed"}
                    <label class="flex items-center gap-2 text-xs text-zinc-400">
                      <span class="w-16 shrink-0">{tr({ es: "Duración", en: "Duration" })}</span>
                      <input
                        type="range"
                        min="0.5"
                        max="15"
                        step="0.5"
                        bind:value={s.sc.activation.seconds}
                        oninput={() => pushScene()}
                        class="flex-1 accent-emerald-500"
                      />
                      <span class="w-10 text-right tabular-nums text-zinc-500"
                        >{s.sc.activation.seconds}s</span
                      >
                    </label>
                  {/if}

                  <p class="text-[11px] leading-relaxed text-zinc-500">
                    {s.sc.activation.mode === "toggle"
                      ? tr({
                          es: "Una pulsación lo enciende y la siguiente lo apaga.",
                          en: "One press shows it, the next hides it.",
                        })
                      : s.sc.activation.mode === "hold"
                        ? tr({
                            es: "Sólo se ve mientras mantienes el botón pulsado.",
                            en: "Only visible while you hold the button down.",
                          })
                        : tr({
                            es: "Una pulsación lo enciende y se apaga solo; volver a pulsar reinicia la cuenta.",
                            en: "One press shows it until the time runs out; pressing again restarts the countdown.",
                          })}
                  </p>
                {/if}
              </div>

              <p class="text-[11px] text-zinc-500">
                {tr({
                  es: "Lente que aumenta lo que hay debajo (como una mira de francotirador). Arrástrala y redimensiónala; los clics pasan al juego. La mirilla se dibuja encima sin aumentar. Mientras haya un visor, el overlay no sale en grabaciones/OBS. Sin botón asignado se ve siempre; en el editor se ve igualmente para poder colocarlo.",
                  en: "Lens that magnifies what's underneath (sniper-style). Drag and resize it; clicks pass to the game. The crosshair draws on top unmagnified. While a scope exists, the overlay is hidden from recordings/OBS. With no button bound it's always on; in the editor it stays visible so you can position it.",
                })}
              </p>
            {:else}
              <!-- controles de mirilla -->
              <div class="flex items-center gap-2">
                <div class="inline-flex rounded-md border border-zinc-700 p-0.5">
                  {#each CH_STYLES as { st, glyph } (st)}
                    <button
                      type="button"
                      onclick={() => {
                        s.ch.style = st;
                        pushScene();
                      }}
                      class="w-7 rounded py-1 text-sm leading-none {s.ch.style === st
                        ? 'bg-emerald-600 text-white'
                        : 'text-zinc-300 hover:bg-zinc-700'}">{glyph}</button
                    >
                  {/each}
                </div>
                <button
                  type="button"
                  title={tr({ es: "Centrar en la pantalla", en: "Center on screen" })}
                  onclick={() => centerPanel(s)}
                  class="rounded border border-zinc-700 p-1.5 text-zinc-300 hover:bg-zinc-700"
                  ><Locate size={14} /></button
                >
                <input
                  type="color"
                  bind:value={s.ch.color}
                  oninput={() => pushScene()}
                  title={tr({ es: "Color", en: "Color" })}
                  class="h-7 w-9 cursor-pointer rounded border border-zinc-700 bg-zinc-900 p-0.5"
                />
              </div>
              <div class="space-y-1.5 text-xs text-zinc-400">
                <label class="flex items-center gap-2">
                  <span class="w-16">{tr({ es: "Tamaño", en: "Size" })}</span>
                  <input
                    type="range"
                    min="16"
                    max="256"
                    step="2"
                    value={s.ch.size}
                    oninput={(e) => setChSize(s, +e.currentTarget.value)}
                    class="flex-1 accent-emerald-500"
                  />
                  <span class="w-10 text-right tabular-nums text-zinc-500">{s.ch.size}px</span>
                </label>
                <label class="flex items-center gap-2">
                  <span class="w-16">{tr({ es: "Grosor", en: "Thickness" })}</span>
                  <input
                    type="range"
                    min="1"
                    max="12"
                    step="0.5"
                    bind:value={s.ch.thickness}
                    oninput={() => pushScene()}
                    class="flex-1 accent-emerald-500"
                  />
                  <span class="w-10 text-right tabular-nums text-zinc-500"
                    >{s.ch.thickness}px</span
                  >
                </label>
                {#if s.ch.style === "cross" || s.ch.style === "x"}
                  <label class="flex items-center gap-2">
                    <span class="w-16">{tr({ es: "Hueco", en: "Gap" })}</span>
                    <input
                      type="range"
                      min="0"
                      max="40"
                      step="1"
                      bind:value={s.ch.gap}
                      oninput={() => pushScene()}
                      class="flex-1 accent-emerald-500"
                    />
                    <span class="w-10 text-right tabular-nums text-zinc-500">{s.ch.gap}px</span>
                  </label>
                {/if}
                <label class="flex items-center gap-2">
                  <span class="w-16">{tr({ es: "Opacidad", en: "Opacity" })}</span>
                  <input
                    type="range"
                    min="0.15"
                    max="1"
                    step="0.05"
                    bind:value={s.ch.alpha}
                    oninput={() => pushScene()}
                    class="flex-1 accent-emerald-500"
                  />
                  <span class="w-10 text-right tabular-nums text-zinc-500"
                    >{Math.round(s.ch.alpha * 100)}%</span
                  >
                </label>
              </div>
              <div class="grid grid-cols-2 gap-1.5">
                <button
                  type="button"
                  onclick={() => {
                    s.ch.dot = !s.ch.dot;
                    pushScene();
                  }}
                  class="flex items-center justify-between rounded border border-zinc-700 px-2 py-1 text-xs {s.ch.dot
                    ? 'bg-emerald-600/20 text-emerald-300'
                    : 'text-zinc-300 hover:bg-zinc-700'}"
                >
                  <span>{tr({ es: "Punto central", en: "Center dot" })}</span>
                  <span class="tabular-nums">{s.ch.dot ? "ON" : "OFF"}</span>
                </button>
                <button
                  type="button"
                  onclick={() => {
                    s.ch.outline = !s.ch.outline;
                    pushScene();
                  }}
                  class="flex items-center justify-between rounded border border-zinc-700 px-2 py-1 text-xs {s.ch.outline
                    ? 'bg-emerald-600/20 text-emerald-300'
                    : 'text-zinc-300 hover:bg-zinc-700'}"
                >
                  <span>{tr({ es: "Contorno", en: "Outline" })}</span>
                  <span class="tabular-nums">{s.ch.outline ? "ON" : "OFF"}</span>
                </button>
              </div>
              <p class="text-[11px] text-zinc-500">
                {tr({
                  es: "La mirilla es solo visual: los clics siempre pasan al juego. Arrástrala en la vista previa para colocarla donde quieras.",
                  en: "The crosshair is visual only: clicks always pass through to the game. Drag it in the preview to place it anywhere.",
                })}
              </p>
            {/if}
          </div>
        {/if}
      </div>

      <!-- window picker -->
      <div>
        <div class="mb-4">
          <span class="text-sm font-medium text-zinc-200"
            >{tr({ es: "Widgets", en: "Widgets" })}</span
          >
          <button
            type="button"
            onclick={addCrosshair}
            class="mt-2 flex w-full items-center gap-2 rounded-md border border-zinc-700 bg-zinc-800/40 px-2 py-1.5 text-left text-xs text-zinc-200 hover:border-emerald-500/50 hover:bg-zinc-800"
          >
            <Crosshair size={14} class="shrink-0 text-emerald-400" />
            <span class="min-w-0 flex-1 truncate font-medium text-zinc-100"
              >{tr({ es: "Añadir mirilla", en: "Add crosshair" })}</span
            >
          </button>
          <button
            type="button"
            onclick={addScope}
            class="mt-1 flex w-full items-center gap-2 rounded-md border border-zinc-700 bg-zinc-800/40 px-2 py-1.5 text-left text-xs text-zinc-200 hover:border-emerald-500/50 hover:bg-zinc-800"
          >
            <ZoomIn size={14} class="shrink-0 text-emerald-400" />
            <span class="min-w-0 flex-1 truncate font-medium text-zinc-100"
              >{tr({ es: "Añadir visor (lupa)", en: "Add scope (magnifier)" })}</span
            >
          </button>
        </div>
        {#if panels.length > 1}
          <div class="mb-4">
            <span class="flex items-center gap-1 text-sm font-medium text-zinc-200"
              ><Layers size={14} /> {tr({ es: "Capas", en: "Layers" })}</span
            >
            <div class="mt-2 space-y-1">
              {#each [...panels].sort((a, b) => b.z - a.z) as p (p.id)}
                <div
                  class="flex items-center gap-1 rounded-md border px-2 py-1 text-xs {selectedId ===
                  p.id
                    ? 'border-emerald-500/60 bg-emerald-600/10 text-emerald-100'
                    : 'border-zinc-700 bg-zinc-800/40 text-zinc-300'}"
                >
                  <button
                    type="button"
                    class="min-w-0 flex-1 truncate text-left"
                    onclick={() => (selectedId = p.id)}
                    >{p.kind === "crosshair" ? "+" : p.kind === "scope" ? "◎" : "▢"}
                    {p.label}</button
                  >
                  <button
                    type="button"
                    title={tr({ es: "Subir capa", en: "Raise layer" })}
                    onclick={() => moveLayer(p, 1)}
                    class="rounded p-0.5 hover:bg-zinc-700"><ArrowUp size={12} /></button
                  >
                  <button
                    type="button"
                    title={tr({ es: "Bajar capa", en: "Lower layer" })}
                    onclick={() => moveLayer(p, -1)}
                    class="rounded p-0.5 hover:bg-zinc-700"><ArrowDown size={12} /></button
                  >
                </div>
              {/each}
            </div>
            <p class="mt-1 text-[11px] text-zinc-500">
              {tr({
                es: "Arriba = encima. Mirillas y visores se dibujan siempre sobre las apps colocadas.",
                en: "Top = above. Crosshairs and scopes always draw over placed apps.",
              })}
            </p>
          </div>
        {/if}
        <div class="mb-2 flex items-center justify-between">
          <span class="text-sm font-medium text-zinc-200"
            >{tr({ es: "Apps", en: "Apps" })}</span
          >
          <button
            type="button"
            onclick={loadWindows}
            class="rounded p-1 text-zinc-400 hover:bg-zinc-700 hover:text-white"
            title={tr({ es: "Actualizar", en: "Refresh" })}><RefreshCw size={15} /></button
          >
        </div>
        <div class="space-y-1">
          {#each windows as w (w.id)}
            <button
              type="button"
              onclick={() => addPanel(w)}
              class="flex w-full items-center gap-2 rounded-md border border-zinc-700 bg-zinc-800/40 px-2 py-1.5 text-left text-xs text-zinc-200 hover:border-emerald-500/50 hover:bg-zinc-800"
            >
              <SquarePlus size={14} class="shrink-0 text-emerald-400" />
              <span class="min-w-0 flex-1 truncate"
                >{#if w.app}<span class="font-medium text-zinc-100">{w.app}</span
                  >{#if w.title}<span class="text-zinc-400"> — {w.title}</span>{/if}{:else}{w.title ||
                    w.id}{/if}</span
              >
              {#if w.protected}
                <TriangleAlert
                  size={13}
                  class="shrink-0 text-amber-400"
                  aria-label={tr({ es: "Contenido protegido", en: "Protected content" })}
                />
              {/if}
            </button>
          {:else}
            <p class="text-xs text-zinc-500">
              {tr({
                es: "No hay ventanas. Abre una app y pulsa actualizar.",
                en: "No windows. Open an app and refresh.",
              })}
            </p>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
