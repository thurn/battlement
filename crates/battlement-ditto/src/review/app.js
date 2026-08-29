"use strict";

const state = {
  result: null,
  screenshots: [],
  selected: 0,
  mode: "split",
  alpha: 0.5,
  zoom: 1,
  panzoom: [],
  syncing: false,
};

const $ = (id) => document.getElementById(id);

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function artifactUrl(path) {
  return `/artifact/${encodeURIComponent(path)}`;
}

function screenshotState(item) {
  const shot = item.step.screenshot;
  if (!shot) return "not available";
  if (shot.status === "unavailable") return "unavailable";
  if (shot.comparison?.status === "mismatch") return "mismatch";
  if (shot.baseline.status === "missing") return "missing baseline";
  if (shot.comparison?.status === "passed") return "passed";
  return "captured";
}

function captured(item) {
  return item.step.screenshot?.status === "captured" ? item.step.screenshot : null;
}

function baselineImage(shot) {
  return shot?.baseline.status === "loaded" ? shot.baseline.image : null;
}

function collectScreenshots(result) {
  const screenshots = [];
  result.scenarios.forEach((scenario, scenarioIndex) => {
    scenario.steps.forEach((step, stepIndex) => {
      if (step.kind === "screenshot") {
        screenshots.push({ scenario, scenarioIndex, step, stepIndex });
      }
    });
  });
  return screenshots;
}

function renderRun() {
  const result = state.result;
  $("run-id").textContent = `${result.run_id} · ${result.profile || "unresolved profile"}`;
  const status = $("run-status");
  status.textContent = result.status.replaceAll("-", " ");
  status.className = `status-pill ${result.status}`;
  $("scenario-count").textContent = String(result.scenarios.length);
  const nav = $("scenario-nav");
  nav.replaceChildren();
  result.scenarios.forEach((scenario, index) => {
    const count = state.screenshots.filter((item) => item.scenarioIndex === index).length;
    const button = element("button", "scenario-button");
    button.type = "button";
    button.dataset.index = String(index);
    button.append(element("span", `dot ${scenario.status}`));
    button.append(element("span", "name", scenario.name));
    button.append(element("span", "screens", String(count)));
    button.addEventListener("click", () => selectScenario(index));
    nav.append(button);
  });
  renderSelection();
}

function selectScenario(index) {
  const match = state.screenshots.findIndex((item) => item.scenarioIndex === index);
  if (match >= 0) {
    state.selected = match;
    renderSelection();
  }
}

function renderSelection() {
  const item = state.screenshots[state.selected];
  document.querySelectorAll(".scenario-button").forEach((button) => {
    button.classList.toggle("active", Number(button.dataset.index) === item?.scenarioIndex);
  });
  if (!item) {
    $("checkpoint-name").textContent = "No screenshot steps";
    $("scenario-name").textContent = "This run contains no reviewable images";
    $("viewer").replaceChildren(emptyState("Nothing to compare", "The result is valid, but no screenshot step was authored."));
    return;
  }
  $("scenario-name").textContent = `${item.scenario.name} · Step ${item.step.index + 1}`;
  $("checkpoint-name").textContent = item.step.screenshot?.checkpoint || item.step.name || "Unavailable screenshot";
  $("position").textContent = `${state.selected + 1} / ${state.screenshots.length}`;
  $("previous").disabled = state.selected === 0;
  $("next").disabled = state.selected === state.screenshots.length - 1;
  renderViewer(item);
  renderDetails(item);
  renderLogs(item);
}

function emptyState(title, detail) {
  const node = element("div", "empty-state");
  node.append(element("strong", "", title));
  node.append(document.createTextNode(detail));
  return node;
}

function image(path, label) {
  const node = document.createElement("img");
  node.src = artifactUrl(path);
  node.alt = label;
  node.draggable = false;
  node.addEventListener("error", () => {
    node.replaceWith(emptyState("Artifact unavailable", `${label} is named by the result but is no longer retained.`));
  });
  node.addEventListener("mousemove", updateCoordinates);
  node.addEventListener("mouseleave", clearCoordinates);
  return node;
}

function pane(label, file) {
  const node = element("div", "pane");
  node.append(element("span", "pane-label", label));
  if (!file) {
    node.append(emptyState(`${label} unavailable`, "The result does not contain this image."));
    return node;
  }
  const target = element("div", "pan-target");
  target.append(image(file.path, label));
  node.append(target);
  return node;
}

function renderViewer(item) {
  destroyPanzoom();
  const viewer = $("viewer");
  viewer.replaceChildren();
  const shot = captured(item);
  if (!shot) {
    const reason = item.step.screenshot?.reason || item.step.status_reason || "The screenshot step was not reached.";
    viewer.append(emptyState("Screenshot unavailable", reason));
    setModeAvailability(false, false);
    return;
  }
  const baseline = baselineImage(shot);
  const diff = shot.comparison?.status === "mismatch" ? shot.comparison.diff : null;
  setModeAvailability(Boolean(baseline), Boolean(diff));
  if (state.mode === "split") renderSplit(viewer, baseline, shot.actual);
  if (state.mode === "swipe") renderSwipe(viewer, baseline, shot.actual);
  if (state.mode === "overlay") renderOverlay(viewer, baseline, shot.actual);
  if (state.mode === "mask") renderMask(viewer, diff, shot.comparison);
  installPanzoom();
}

function renderSplit(viewer, baseline, actual) {
  const split = element("div", "split-view");
  split.append(pane("Baseline", baseline), pane("Actual", actual));
  viewer.append(split);
}

function renderSwipe(viewer, baseline, actual) {
  if (!baseline) {
    viewer.append(emptyState("Swipe unavailable", "This screenshot has no loaded baseline."));
    return;
  }
  const frame = element("div", "single-view");
  const target = element("div", "pan-target");
  const slider = document.createElement("img-comparison-slider");
  const before = image(baseline.path, "Baseline");
  const after = image(actual.path, "Actual");
  before.slot = "first";
  after.slot = "second";
  slider.append(before, after);
  target.append(slider);
  frame.append(target);
  viewer.append(frame);
}

function renderOverlay(viewer, baseline, actual) {
  if (!baseline) {
    viewer.append(emptyState("Overlay unavailable", "This screenshot has no loaded baseline."));
    return;
  }
  const frame = element("div", "single-view");
  const target = element("div", "pan-target overlay-stack");
  target.append(image(baseline.path, "Baseline"));
  const actualImage = image(actual.path, "Actual");
  actualImage.style.opacity = String(state.alpha);
  target.append(actualImage);
  frame.append(target);
  viewer.append(frame);
}

function renderMask(viewer, diff, comparison) {
  if (!diff) {
    const detail = comparison?.status === "passed"
      ? "The authoritative result says this change is within tolerance, so no red mask was retained."
      : "No comparison mask is available for this checkpoint.";
    viewer.append(emptyState("No red mask", detail));
    return;
  }
  const frame = element("div", "single-view");
  const target = element("div", "pan-target");
  target.append(image(diff.path, "ODiff red mask"));
  frame.append(target);
  viewer.append(frame);
}

function setModeAvailability(hasBaseline, hasDiff) {
  document.querySelectorAll("[data-mode]").forEach((button) => {
    const mode = button.dataset.mode;
    button.disabled = (mode === "swipe" || mode === "overlay") && !hasBaseline;
    button.title = button.disabled ? "A loaded baseline is required" : "";
    button.classList.toggle("active", mode === state.mode);
  });
  $("alpha-control").hidden = state.mode !== "overlay";
  const mask = document.querySelector("[data-mode=mask]");
  mask.title = hasDiff ? "" : "Passing or unavailable comparisons have no retained mask";
}

function installPanzoom() {
  document.querySelectorAll(".pan-target").forEach((target) => {
    const instance = Panzoom(target, { minScale: 1, maxScale: 8, step: 1 });
    target.addEventListener("panzoomchange", (event) => synchronizePanzoom(instance, event.detail));
    state.panzoom.push(instance);
  });
  $("viewer").onwheel = (event) => {
    event.preventDefault();
    setZoom(state.zoom + (event.deltaY < 0 ? 1 : -1));
  };
  setZoom(state.zoom);
}

function synchronizePanzoom(source, detail) {
  if (state.syncing || !detail.originalEvent) return;
  const scale = Math.max(1, Math.min(8, Math.round(detail.scale)));
  state.zoom = scale;
  state.syncing = true;
  state.panzoom.forEach((instance) => {
    if (instance === source) return;
    instance.zoom(scale, { animate: false });
    instance.pan(detail.x, detail.y, { animate: false, force: true });
  });
  if (detail.scale !== scale) source.zoom(scale, { animate: false });
  state.syncing = false;
  $("zoom-reset").textContent = `${scale}×`;
}

function setZoom(value) {
  state.zoom = Math.max(1, Math.min(8, Math.round(value)));
  state.syncing = true;
  state.panzoom.forEach((instance) => instance.zoom(state.zoom, { animate: false }));
  state.syncing = false;
  $("zoom-reset").textContent = `${state.zoom}×`;
}

function resetView() {
  state.zoom = 1;
  state.syncing = true;
  state.panzoom.forEach((instance) => instance.reset({ animate: false }));
  state.syncing = false;
  $("zoom-reset").textContent = "1×";
}

function destroyPanzoom() {
  state.panzoom.forEach((instance) => instance.destroy());
  state.panzoom = [];
  $("viewer").onwheel = null;
}

function updateCoordinates(event) {
  const imageNode = event.currentTarget;
  if (!imageNode.naturalWidth) return;
  const bounds = imageNode.getBoundingClientRect();
  const x = Math.floor((event.clientX - bounds.left) * imageNode.naturalWidth / bounds.width);
  const y = Math.floor((event.clientY - bounds.top) * imageNode.naturalHeight / bounds.height);
  $("coordinates").value = `x ${x} · y ${y}`;
}

function clearCoordinates() {
  $("coordinates").value = "x — · y —";
}

function addDetail(list, term, value) {
  list.append(element("dt", "", term), element("dd", "", value));
}

function renderDetails(item) {
  const comparison = $("comparison-details");
  comparison.replaceChildren();
  const shot = captured(item);
  addDetail(comparison, "Status", screenshotState(item));
  if (shot) addDetail(comparison, "Dimensions", `${shot.actual.width} × ${shot.actual.height}`);
  if (shot?.comparison) {
    const changed = shot.comparison.changed_pixels;
    const total = shot.comparison.total_pixels;
    addDetail(comparison, "Changed", `${changed.toLocaleString()} / ${total.toLocaleString()} (${(changed / total * 100).toFixed(4)}%)`);
    addDetail(comparison, "Pixel threshold", shot.comparison.settings.threshold);
    addDetail(comparison, "Changed limit", `${shot.comparison.settings.max_changed_percent}%`);
    addDetail(comparison, "Anti-alias", shot.comparison.settings.anti_alias ? "ignored" : "counted");
  }
  if (item.step.screenshot?.status === "unavailable") {
    addDetail(comparison, "Reason", item.step.screenshot.reason);
  }

  const timings = $("timing-details");
  timings.replaceChildren();
  addDetail(timings, "Step", `${item.step.duration_ms.toLocaleString()} ms`);
  addDetail(timings, "Scenario", `${item.scenario.duration_ms.toLocaleString()} ms`);
  const labels = {
    startup_ms: "Startup",
    reset_ms: "Reset",
    baseline_download_ms: "Baseline download",
    comparison_ms: "Comparison",
    media_ms: "Media",
    durability_ms: "Durability",
  };
  Object.entries(labels).forEach(([key, label]) => {
    const value = item.scenario.timings[key];
    if (value !== null) addDetail(timings, label, `${value.toLocaleString()} ms`);
  });
}

async function renderLogs(item) {
  const selected = state.selected;
  const span = item.scenario.logs;
  const output = $("logs");
  if (!span) {
    output.textContent = "No correlated logs for this scenario.";
    $("log-range").textContent = "";
    return;
  }
  $("log-range").textContent = `${span.first_sequence}–${span.last_sequence}${span.complete ? "" : " · partial"}`;
  output.textContent = "Loading correlated records…";
  try {
    const response = await fetch(artifactUrl(span.path));
    if (!response.ok) throw new Error(await response.text());
    const records = (await response.text()).trim().split("\n").filter(Boolean).map((line) => JSON.parse(line));
    if (selected !== state.selected) return;
    const correlated = records.filter((record) => {
      const sequence = record.sequence;
      return sequence >= span.first_sequence && sequence <= span.last_sequence;
    });
    output.textContent = correlated.length
      ? correlated.map(formatLog).join("\n")
      : "The retained log contains no records in this scenario span.";
  } catch (error) {
    if (selected === state.selected) output.textContent = `Log artifact unavailable: ${error.message}`;
  }
}

function formatLog(record) {
  const sequence = String(record.sequence ?? "—").padStart(5, "0");
  const level = String(record.level ?? record.kind ?? "event").toUpperCase();
  return `${sequence}  ${level.padEnd(9)}  ${record.message ?? JSON.stringify(record)}`;
}

function bindControls() {
  $("previous").addEventListener("click", () => move(-1));
  $("next").addEventListener("click", () => move(1));
  $("zoom-out").addEventListener("click", () => setZoom(state.zoom - 1));
  $("zoom-in").addEventListener("click", () => setZoom(state.zoom + 1));
  $("zoom-reset").addEventListener("click", resetView);
  $("alpha").addEventListener("input", (event) => {
    state.alpha = Number(event.target.value) / 100;
    $("alpha-value").textContent = `${event.target.value}%`;
    const actual = document.querySelector(".overlay-stack img + img");
    if (actual) actual.style.opacity = String(state.alpha);
  });
  document.querySelectorAll("[data-mode]").forEach((button) => {
    button.addEventListener("click", () => {
      state.mode = button.dataset.mode;
      renderSelection();
    });
  });
  document.addEventListener("keydown", (event) => {
    if (event.target.matches("input")) return;
    if (event.key === "ArrowLeft") move(-1);
    if (event.key === "ArrowRight") move(1);
  });
}

function move(delta) {
  const next = Math.max(0, Math.min(state.screenshots.length - 1, state.selected + delta));
  if (next !== state.selected) {
    state.selected = next;
    renderSelection();
  }
}

async function start() {
  bindControls();
  const response = await fetch("/api/result");
  if (!response.ok) throw new Error(await response.text());
  state.result = await response.json();
  state.screenshots = collectScreenshots(state.result);
  renderRun();
}

start().catch((error) => {
  $("fatal").hidden = false;
  $("fatal").textContent = `Review could not load: ${error.message}`;
  document.querySelector(".shell").hidden = true;
});
