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
  key: string;
  at: string;
  level: Level;
  title: string;
  body: string;
}

interface Notice {
  title: string;
  body: string;
  level: Level;
}

const SWITCHES = [
  ["backup_success", "Notify on successful backup"],
  ["backup_problems", "Notify on backup problems"],
  ["restore_success", "Notify when a save downloads from the cloud"],
  ["restore_problems", "Notify on download problems"],
  ["game_activity", "Notify when a game starts or stops"],
  ["updates", "Notify when an update needs you"],
] as const;

type SwitchName = (typeof SWITCHES)[number][0];

interface Slot {
  save_id: string;
  name: string;
  path: string;
  running: boolean;
  armed: boolean;
}

type Switches = Record<SwitchName, boolean>;

interface State {
  connected: boolean;
  error: string;
  running: boolean;
  problem: string;
  daemon_version: string;
  server: string;
  watched: number;
  slots: Slot[];
  recent: Item[];
  // Null when the service is too old to keep them.
  switches: Switches | null;
}

const refresh = callable<[], State>("refresh");
const syncNow = callable<[], string>("sync_now");
const setSwitch = callable<[name: SwitchName, enabled: boolean], string>("set_switch");

function ago(at: string): string {
  const secs = Math.max(0, (Date.now() - Date.parse(at)) / 1000);
  if (secs < 60) return "just now";
  if (secs < 3600) return `${Math.floor(secs / 60)} min ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)} h ago`;
  return `${Math.floor(secs / 86400)} d ago`;
}

function host(server: string): string {
  return server.replace(/^https?:\/\//, "").replace(/\/$/, "");
}

function Content() {
  const [state, setState] = useState<State | null>(null);
  const [failed, setFailed] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    // A backend that failed to load rejects every call; say so instead of
    // leaving an empty panel.
    const load = () =>
      refresh()
        .then((s) => {
          setState(s);
          setFailed("");
        })
        .catch((e) => setFailed(String(e)));
    load();
    const onState = addEventListener<[State]>("hoard_state", setState);
    const onEvent = addEventListener<[Item]>("hoard_event", load);
    return () => {
      removeEventListener("hoard_state", onState);
      removeEventListener("hoard_event", onEvent);
    };
  }, []);

  if (!state) {
    return (
      <PanelSection>
        <PanelSectionRow>
          <Field label={failed ? "The plugin is not responding" : "Loading"} description={failed} />
        </PanelSectionRow>
      </PanelSection>
    );
  }

  const saves = `${state.watched} ${state.watched === 1 ? "save" : "saves"}`;
  const playing = state.slots.filter((s) => s.running).map((s) => s.name);

  return (
    <>
      <PanelSection>
        <PanelSectionRow>
          <Field
            label={!state.connected ? "Sleeping" : state.running ? "Syncing" : "Paused"}
            description={
              !state.connected
                ? state.error
                : state.running
                  ? [saves, host(state.server)].filter(Boolean).join(" to ")
                  : state.problem
            }
          />
        </PanelSectionRow>
        {playing.length > 0 && (
          <PanelSectionRow>
            <Field label="Playing" description={playing.join(", ")} />
          </PanelSectionRow>
        )}
        <PanelSectionRow>
          <Field
            bottomSeparator="none"
            description="Press here if you have a upload problem or just want do it manually before a shutdown."
          />
        </PanelSectionRow>
        <PanelSectionRow>
          <ButtonItem
            layout="below"
            disabled={!state.connected || !state.running || busy}
            onClick={async () => {
              setBusy(true);
              const error = await syncNow();
              setBusy(false);
              if (error) toaster.toast({ title: "Upload failed", body: error });
            }}
          >
            Manual Upload
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>

      <PanelSection title="Recent">
        {state.recent.length === 0 ? (
          <PanelSectionRow>
            <Field description="Nothing yet." />
          </PanelSectionRow>
        ) : (
          state.recent.slice(0, 8).map((item) => (
            <PanelSectionRow key={item.key}>
              <Field label={item.title} description={`${item.body}, ${ago(item.at)}`} />
            </PanelSectionRow>
          ))
        )}
      </PanelSection>

      {state.switches && (
        <PanelSection title="Notifications">
          {SWITCHES.map(([name, label]) => (
            <PanelSectionRow key={name}>
              <ToggleField
                label={label}
                checked={state.switches![name]}
                onChange={async (value: boolean) => {
                  const error = await setSwitch(name, value);
                  if (error) toaster.toast({ title: "Hoard", body: error });
                }}
              />
            </PanelSectionRow>
          ))}
        </PanelSection>
      )}
    </>
  );
}

export default definePlugin(() => {
  const listener = addEventListener<[Notice]>("hoard_notice", (notice) => {
    toaster.toast({
      title: notice.title,
      body: notice.body,
      critical: notice.level === "error",
    });
  });

  return {
    name: "Hoard",
    titleView: <div className={staticClasses.Title}>Hoard</div>,
    content: <Content />,
    icon: <FaSave />,
    onDismount() {
      removeEventListener("hoard_notice", listener);
    },
  };
});
