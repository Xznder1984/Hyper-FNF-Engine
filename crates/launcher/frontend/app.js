// Hyper Engine launcher UI (vanilla JS, no build step).
// This file is the checked-in JS fallback; the canonical UI source lives in
// haxe/ and can be compiled with:  haxe build.hxml  ->  overwrites app.js.
"use strict";

const $ = (id) => document.getElementById(id);

async function call(cmd, args = {}) {
  const api = window.__TAURI__?.core;
  if (!api) throw new Error("Tauri bridge unavailable");
  return api.invoke(cmd, args);
}

function setStatus(msg) {
  $("status").textContent = msg || "";
}

function alert(msg) {
  const el = $("alerts");
  if (msg) {
    el.textContent = msg;
    el.hidden = false;
  } else {
    el.hidden = true;
  }
}

function appendProgress(msg) {
  const list = $("progress-list");
  const item = document.createElement("li");
  item.textContent = msg;
  list.appendChild(item);
  list.scrollTop = list.scrollHeight;
  $("progress-box").hidden = false;
}

async function refresh() {
  try {
    setStatus("Loading engines…");
    const engines = await call("engines");
    renderEngines(engines);
    setStatus("");
  } catch (e) {
    setStatus("");
    alert(String(e));
  }
}

async function loadSettings() {
  try {
    const s = await call("settings_get");
    $("assets-path").value = s.game_assets || "";
    $("hud-toggle").checked = !!(s.sarahud && s.sarahud.enabled);
  } catch (e) {
    alert(String(e));
  }
}

function renderEngines(list) {
  const tbody = $("engine-rows");
  tbody.textContent = "";
  for (const e of list) {
    const tr = document.createElement("tr");
    const tdName = document.createElement("td");
    tdName.textContent = e.name;
    const tdLic = document.createElement("td");
    tdLic.textContent = e.license || "—";
    const tdInst = document.createElement("td");
    tdInst.textContent = e.installed.length ? e.installed.join(", ") : "not installed";
    const tdSrc = document.createElement("td");
    if (e.homepage) {
      const a = document.createElement("a");
      a.href = e.homepage;
      a.textContent = e.repo || "source";
      a.target = "_blank";
      a.rel = "noopener noreferrer";
      tdSrc.appendChild(a);
    } else {
      tdSrc.textContent = "—";
    }
    tr.append(tdName, tdLic, tdInst, tdSrc);
    tbody.appendChild(tr);
  }
}

function renderDetection(d) {
  $("det-engine").textContent = d.engine_id || "none";
  $("det-confidence").textContent = d.confidence;
  $("det-evidence").textContent = d.evidence.length ? d.evidence.join("; ") : "—";
  $("det-candidates").textContent = d.candidates.length
    ? d.candidates.map((c) => `${c.engine_id} (${c.score})`).join(", ")
    : "—";
  $("detection").hidden = false;
  $("btn-launch").disabled = !d.engine_id;
}

$("mod-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const path = $("mod-path").value.trim();
  if (!path) {
    $("path-error").textContent = "Enter a mod folder path.";
    return;
  }
  $("path-error").textContent = "";
  $("btn-detect").disabled = true;
  setStatus("Detecting…");
  try {
    const d = await call("detect_mod", { modDir: path });
    renderDetection(d);
    setStatus(d.engine_id ? "Detection complete." : "Could not detect an engine.");
    if (!d.engine_id) alert("No engine detected. Install content from the engine's own creators and retry.");
    else alert("");
  } catch (e) {
    setStatus("");
    alert(String(e));
  } finally {
    $("btn-detect").disabled = false;
  }
});

$("btn-launch").addEventListener("click", async () => {
  const path = $("mod-path").value.trim();
  $("btn-launch").disabled = true;
  setStatus("Preparing…");
  try {
    const outcome = await call("launch_mod", { modDir: path });
    setStatus(`Launched pid ${outcome.pid} (${outcome.exe}). The game keeps running.`);
  } catch (e) {
    setStatus("");
    alert(String(e));
  } finally {
    $("btn-launch").disabled = false;
  }
});

$("assets-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  try {
    await call("set_game_assets", { path: $("assets-path").value.trim() });
    alert("Assets path saved.");
  } catch (e) {
    alert(String(e));
  }
});

$("hud-toggle").addEventListener("change", async () => {
  try {
    await call("sarahud_enabled", { enabled: $("hud-toggle").checked });
    alert("SaraHUD preference saved.");
  } catch (e) {
    alert(String(e));
  }
});

$("btn-refresh").addEventListener("click", refresh);

for (const btn of document.querySelectorAll("button[data-href]")) {
  btn.addEventListener("click", () => {
    call("open_external", { url: btn.dataset.href }).catch(() => {});
  });
}

(async function init() {
  const api = window.__TAURI__;
  if (api?.event) {
    api.event.listen("hyper://progress", (ev) => appendProgress(String(ev.payload)));
  }
  await Promise.all([refresh(), loadSettings()]);
})();