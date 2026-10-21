import {
  ButtonItem,
  Field,
  PanelSection,
  PanelSectionRow,
  ToggleField,
  staticClasses,
} from "@decky/ui";
import {
  addEventListener,
  callable,
  definePlugin,
  removeEventListener,
  toaster,
} from "@decky/api";
import { useEffect, useState } from "react";
import { FaSave } from "react-icons/fa";

type Level = "ok" | "info" | "warn" | "error";

interface Item {
  seq: number;
  at: string;
  level: Level;
  title: string;
  body: string;
  toast: boolean;
}

interface Slot {
  save_id: string;
  name: string;
  path: string;
  running: boolean;
  armed: boolean;
}

interface State {
  connected: boolean;
  error: string;
  daemon_version: string;
  server: string;
  watched: number;
  slots: Slot[];
  recent: Item[];
  debug_toasts: boolean;
}

const refresh = callable<[], State>("refresh");
const syncNow = callable<[], string>("sync_now");
const setDebugToasts = callable<[enabled: boolean], void>("set_debug_toasts");

const DOT: Record<Level, string> = {
  ok: "#4ade80",
  info: "#9ca3af",
  warn: "#facc15",
  error: "#f87171",
};

function ago(at: string): string {
  const secs = Math.max(0, (Date.now() - Date.parse(at)) / 1000);
  if (secs < 60) return "now";
  if (secs < 3600) return `${Math.floor(secs / 60)} min`;
  if (secs < 86400) return `${Math.floor(secs / 3600)} h`;
  return `${Math.floor(secs / 86400)} d`;
}

function Content() {
  const [state, setState] = useState<State | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refresh().then(setState);
    const onState = addEventListener<[State]>("hoard_state", setState);
    const onEvent = addEventListener<[Item]>("hoard_event", () => {
      refresh().then(setState);
    });
    return () => {
      removeEventListener("hoard_state", onState);
      removeEventListener("hoard_event", onEvent);
    };
  }, []);

  if (!state) return null;

  const running = state.slots.filter((s) => s.running);

  return (
    <>
      <PanelSection title="Sync service">
        <PanelSectionRow>
          <Field
            label={state.connected ? "Running" : "Not running"}
            description={
              state.connected
                ? `${state.watched} ${state.watched === 1 ? "save" : "saves"} watched · hoardd ${state.daemon_version}`
                : state.error
            }
          />
        </PanelSectionRow>
        {state.connected && state.server && (
          <PanelSectionRow>
            <Field label="Server" description={state.server} />
          </PanelSectionRow>
        )}
        {running.map((s) => (
          <PanelSectionRow key={s.path}>
            <Field label="Playing" description={s.name} />
          </PanelSectionRow>
        ))}
        <PanelSectionRow>
          <ButtonItem
            layout="below"
            disabled={!state.connected || busy}
            onClick={async () => {
              setBusy(true);
              const error = await syncNow();
              setBusy(false);
              toaster.toast({
                title: "Hoard",
                body: error ? `Sync failed: ${error}` : "Backing up every save that changed",
              });
            }}
          >
            {busy ? "Syncing..." : "Sync now"}
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>

      <PanelSection title="Recent">
        {state.recent.length === 0 && (
          <PanelSectionRow>
            <Field label="Nothing yet" description="Uploads and restores show up here." />
          </PanelSectionRow>
        )}
        {state.recent.slice(0, 12).map((item) => (
          <PanelSectionRow key={item.seq}>
            <Field
              label={
                <span>
                  <span style={{ color: DOT[item.level], marginRight: 6 }}>●</span>
                  {item.title}
                </span>
              }
              description={`${item.body} · ${ago(item.at)}`}
            />
          </PanelSectionRow>
        ))}
      </PanelSection>

      <PanelSection title="Debug">
        <PanelSectionRow>
          <ToggleField
            label="Notify when a game is detected"
            description="Shows whether Hoard sees your game start and stop."
            checked={state.debug_toasts}
            onChange={async (value: boolean) => {
              await setDebugToasts(value);
              setState({ ...state, debug_toasts: value });
            }}
          />
        </PanelSectionRow>
      </PanelSection>
    </>
  );
}

export default definePlugin(() => {
  const listener = addEventListener<[Item]>("hoard_event", (item) => {
    if (!item.toast) return;
    toaster.toast({
      title: item.title,
      body: item.body,
      critical: item.level === "error",
    });
  });

  return {
    name: "Hoard",
    titleView: <div className={staticClasses.Title}>Hoard</div>,
    content: <Content />,
    icon: <FaSave />,
    onDismount() {
      removeEventListener("hoard_event", listener);
    },
  };
});
