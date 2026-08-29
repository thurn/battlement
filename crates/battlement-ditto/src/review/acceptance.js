"use strict";

const acceptanceState = {
  capability: null,
  selected: new Map(),
  busy: false,
  notice: null,
};

function initializeAcceptance(capability) {
  acceptanceState.capability = capability;
  $("accept-current").addEventListener("change", toggleCurrentAcceptance);
  $("accept-selected").addEventListener("click", acceptSelected);
  renderAcceptanceSelection(state.screenshots[state.selected]);
}

function acceptanceKey(item) {
  const shot = captured(item);
  return shot ? `${item.scenario.name}\u0000${shot.checkpoint}` : null;
}

function reviewSelection(item) {
  if (!item) return null;
  const shot = captured(item);
  if (!shot) return null;
  return {
    profile: state.result.profile,
    scenario: item.scenario.name,
    checkpoint: shot.checkpoint,
    width: shot.actual.width,
    height: shot.actual.height,
    actual_sha256: shot.actual.sha256,
  };
}

function renderAcceptanceSelection(item) {
  const capability = acceptanceState.capability;
  const checkbox = $("accept-current");
  const button = $("accept-selected");
  const selection = reviewSelection(item);
  checkbox.disabled = !capability?.enabled || !selection || acceptanceState.busy;
  checkbox.checked = selection
    ? acceptanceState.selected.has(acceptanceKey(item))
    : false;
  button.disabled = !capability?.enabled || acceptanceState.selected.size === 0 || acceptanceState.busy;
  button.textContent = acceptanceState.busy
    ? "Accepting…"
    : `Accept selected (${acceptanceState.selected.size})`;
  if (!capability) return;
  if (!capability.enabled) {
    setAcceptanceMessage(capability.reason || "Baseline write credentials are unavailable.", "error");
  } else if (!acceptanceState.busy) {
    const message = acceptanceState.notice || {
      text: acceptanceState.selected.size
        ? `${acceptanceState.selected.size} screenshot${acceptanceState.selected.size === 1 ? "" : "s"} selected for one atomic update.`
        : "Select screenshots across scenarios, then accept them together.",
      className: "",
    };
    setAcceptanceMessage(message.text, message.className);
  }
}

function toggleCurrentAcceptance() {
  const item = state.screenshots[state.selected];
  const selection = reviewSelection(item);
  if (!selection) return;
  const key = acceptanceKey(item);
  acceptanceState.notice = null;
  if ($("accept-current").checked) acceptanceState.selected.set(key, selection);
  else acceptanceState.selected.delete(key);
  renderAcceptanceSelection(item);
}

async function acceptSelected() {
  acceptanceState.busy = true;
  renderAcceptanceSelection(state.screenshots[state.selected]);
  const body = {
    request_id: crypto.randomUUID(),
    run_id: state.result.run_id,
    lock_sha256: state.result.lock_sha256,
    selections: [...acceptanceState.selected.values()],
  };
  try {
    const response = await fetch(`/api/accept/${acceptanceState.capability.token}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.message || `Acceptance failed (${response.status})`);
    const resultResponse = await fetch("/api/result");
    if (!resultResponse.ok) throw new Error(await resultResponse.text());
    state.result = await resultResponse.json();
    state.screenshots = collectScreenshots(state.result);
    state.selected = Math.min(state.selected, Math.max(0, state.screenshots.length - 1));
    acceptanceState.selected.clear();
    acceptanceState.notice = {
      text: `Accepted into ${payload.comparison_run_id}.`,
      className: "success",
    };
    renderRun();
  } catch (error) {
    acceptanceState.notice = { text: error.message, className: "error" };
  } finally {
    acceptanceState.busy = false;
    renderAcceptanceSelection(state.screenshots[state.selected]);
  }
}

function setAcceptanceMessage(message, className) {
  $("acceptance-message").textContent = message;
  $("acceptance-bar").className = `acceptance-bar ${className}`.trim();
}
