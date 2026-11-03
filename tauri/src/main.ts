import { invoke } from "@tauri-apps/api/core";
import "./style.css";

type Measurement = {
  at: number;
  download: number | null;
  upload: number | null;
  latency: number | null;
  healthy: boolean;
};
type Network = {
  uuid: string;
  name: string;
  ssid: string;
  signal: number;
  band: string;
  active: boolean;
  measurement: Measurement | null;
};
type Snapshot = {
  enabled: boolean;
  updatedAt: number;
  interface: string;
  current: string;
  phase: string;
  message: string;
  networks: Network[];
  downloadRate: number;
  uploadRate: number;
  events: { at: number; text: string }[];
};
const app = document.querySelector<HTMLDivElement>("#app")!;
app.innerHTML = `<main><header><div class="mark" aria-hidden="true">⇄</div><div><h1>Internet Swap</h1><p>Your Wi-Fi, with a little backup.</p></div><button id="theme" class="theme-button" aria-label="Toggle light or dark mode"><span class="to-light">☀</span><span class="to-dark">☾</span></button></header><section class="control"><div><h2>Automatic switching</h2><p id="mode">Connecting to background service…</p></div><button id="toggle" class="switch" role="switch" aria-checked="false" aria-label="Automatic switching" disabled><span></span></button></section><section class="connection"><div class="eyebrow"><span id="dot" class="dot"></span><span id="phase">CONNECTING</span></div><h2 id="current">Looking for Wi-Fi</h2><p id="message" role="status" aria-live="polite">Starting the background service.</p><div class="rates"><div><span>↓ Live download</span><strong id="down">—</strong></div><div><span>↑ Live upload</span><strong id="up">—</strong></div></div></section><section><div class="section-head"><h2>Saved & available</h2><button id="test" class="text-button" disabled>Test networks ↗</button></div><p class="hint">Signal updates in the background. Speed results show when last tested.</p><div id="networks"></div></section><p id="error" class="error" role="alert" hidden></p><details><summary>Recent activity</summary><ol id="events"></ol></details><footer>One adapter, one connection. Switching can interrupt active requests.<br>USB tethering is outside this app’s selection logic.</footer></main><dialog id="confirm"><h2 id="confirm-title"></h2><p id="confirm-text"></p><div class="dialog-actions"><button id="cancel">Cancel</button><button id="accept" class="primary">Continue</button></div></dialog>`;
const get = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const toggle = get<HTMLButtonElement>("toggle"),
  test = get<HTMLButtonElement>("test");
let state: Snapshot | null = null,
  pending = false,
  fetching = false,
  wasOffline = false;
function text(id: string, value: string) {
  get(id).textContent = value;
}
function rate(n: number) {
  return n >= 1e6
    ? `${(n / 1e6).toFixed(2)} MB/s`
    : `${(n / 1000).toFixed(1)} KB/s`;
}
function speed(n: number | null) {
  return n == null ? "—" : `${n.toFixed(1)} Mbps`;
}
function age(at: number) {
  const m = Math.max(0, Math.floor((Date.now() / 1000 - at) / 60));
  return m < 1
    ? "just now"
    : m < 60
      ? `${m}m ago`
      : `${Math.floor(m / 60)}h ago`;
}
function error(message: string) {
  text("error", message);
  get("error").hidden = !message;
}
function render(s: Snapshot) {
  state = s;
  toggle.disabled = pending;
  toggle.setAttribute("aria-checked", String(s.enabled));
  text(
    "mode",
    s.enabled
      ? "On, even when this window is closed."
      : "Off. Your current connection stays as it is.",
  );
  const busy = ["testing", "switching", "recovering", "measuring"].includes(
    s.phase,
  );
  test.disabled = pending || busy || !s.enabled || !s.networks.length;
  text("phase", s.phase.toUpperCase());
  get("dot").classList.toggle("paused", !s.enabled);
  const current = s.networks.find((n) => n.uuid === s.current);
  text(
    "current",
    current?.name || (s.current ? "Connected Wi-Fi" : "No Wi-Fi connected"),
  );
  text("message", s.message);
  text("down", rate(s.downloadRate));
  text("up", rate(s.uploadRate));
  const list = get("networks");
  list.replaceChildren();
  if (!s.networks.length) {
    const p = document.createElement("p");
    p.className = "empty";
    p.textContent =
      "No saved, secured Wi-Fi networks are visible. Connect through your system’s Wi-Fi settings first.";
    list.append(p);
  }
  for (const n of s.networks) {
    const row = document.createElement("article");
    row.className = "network";
    const top = document.createElement("div");
    top.className = "network-top";
    const name = document.createElement("strong");
    name.textContent = n.name;
    const button = document.createElement("button");
    button.className = n.active ? "connected" : "text-button";
    button.textContent = n.active ? "Connected" : "Connect";
    button.disabled = n.active || busy || pending || !s.enabled;
    button.setAttribute(
      "aria-label",
      n.active ? `${n.name}, connected` : `Connect to ${n.name}`,
    );
    button.onclick = () =>
      confirmAction(
        "Switch Wi-Fi?",
        `Connect to ${n.name}. Active requests may be interrupted. If internet checks fail, I’ll try the previous connection.`,
        `connect ${n.uuid}`,
      );
    top.append(name, button);
    const meta = document.createElement("p");
    meta.className = "network-meta";
    meta.textContent = `${n.band} · ${n.signal}% signal`;
    const measurement = document.createElement("p");
    measurement.className = "measurement";
    const m = n.measurement;
    measurement.textContent = m
      ? `↓ ${speed(m.download)}   ↑ ${speed(m.upload)} · ${age(m.at)}${!m.healthy ? " · internet check failed" : ""}`
      : "Not speed-tested yet";
    row.append(top, meta, measurement);
    list.append(row);
  }
  const events = get("events");
  events.replaceChildren();
  for (const event of s.events.slice(0, 8)) {
    const li = document.createElement("li");
    const time = document.createElement("time");
    time.textContent = age(event.at);
    const span = document.createElement("span");
    span.textContent = event.text;
    li.append(time, span);
    events.append(li);
  }
}
async function refresh() {
  if (fetching) return;
  fetching = true;
  try {
    const s = await invoke<Snapshot>("status");
    render(s);
    if (wasOffline) {
      error("");
      wasOffline = false;
    }
  } catch {
    wasOffline = true;
    toggle.disabled = true;
    test.disabled = true;
    text("phase", "SERVICE UNAVAILABLE");
    text(
      "message",
      "The background service is unavailable. Reopen the app to start it again.",
    );
    text("down", "—");
    text("up", "—");
  } finally {
    fetching = false;
  }
}
async function control(action: string) {
  if (pending) return;
  pending = true;
  if (state) render(state);
  error("");
  try {
    const result = await invoke<{ error?: string }>("control", { action });
    if (result.error) throw Error(result.error);
  } catch (e) {
    error(String(e));
  } finally {
    pending = false;
    await refresh();
  }
}
toggle.onclick = () => {
  if (state) void control(state.enabled ? "disable" : "enable");
};
const dialog = get<HTMLDialogElement>("confirm");
let confirmedAction = "";
function confirmAction(title: string, description: string, action: string) {
  text("confirm-title", title);
  text("confirm-text", description);
  confirmedAction = action;
  dialog.showModal();
  get<HTMLButtonElement>("cancel").focus();
}
test.onclick = () =>
  confirmAction(
    "Test available networks?",
    "This joins each saved, visible secured Wi-Fi in turn, using about 1.25 MiB per successful test. Wi-Fi will disconnect briefly. Afterwards, the previous connection is restored; automatic selection can then use these results.",
    "test",
  );
get("cancel").onclick = () => dialog.close();
get("accept").onclick = () => {
  dialog.close();
  void control(confirmedAction);
};
void refresh();
setInterval(() => {
  if (!document.hidden) void refresh();
}, 1500);
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) void refresh();
});

get("theme").onclick = () => {
  const mode =
    document.documentElement.dataset.mode === "dark" ? "light" : "dark";
  document.documentElement.dataset.mode = mode;
  try {
    localStorage.setItem("internet-swap-mode", mode);
  } catch {
    error(
      "Appearance changed for this session. The preference could not be saved.",
    );
  }
};
