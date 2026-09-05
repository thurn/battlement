---
name: battlement-web
description: Prepare a Battlement web review demo and Cloudflare Quick Tunnel, verify web-specific behavior, or deploy the sample site when publishing is requested.
---

# Web review and publishing

For web-visible work, follow the global wt skill's durable-service procedure.
From the worktree, stage build inputs, then run
`python3 scripts/prepare-web-demo.py <sample>` (add `--release` when needed).
Use the returned build directory with
`python3 scripts/serve_web.py --directory <build-directory> --port <port>`;
this server supplies the headers required by threaded Unity Web players.

Choose a verified-free non-default port. Run the server as a named durable
service (launchd on macOS), then run a second named durable service executing
`cloudflared tunnel --no-autoupdate --url http://127.0.0.1:<port>`.
Record exact service labels, logs, worktree, port, and process identities.
Do not rely on a turn-scoped terminal or background shell for persistence.

Verify both the local review URL and the generated `https://*.trycloudflare.com`
URL load the intended screen and exercise its first relevant control. Inspect
console and failed requests through asset loading; a loader or canvas is not
proof the application works. Keep tunnel failures distinct from native results.

Use only the globally configured Playwright MCP for browser automation. It is
an isolated-context client of the singleton at `http://localhost:8931/mcp`.
If unavailable, run `playwright-mcp-service start` and retry. Never directly
launch a browser/automation CLI or request a shared context. Prefer snapshots,
DOM state, and locators; take screenshots when appearance is relevant.
Close this task's browser context after QA, leaving the shared service running.

Include the exact verified public review URL and a short interaction walkthrough
in the review handoff. Keep both durable services running through review.
Immediately before authorizing promotion, stop only the recorded services and
confirm the port is free and the tunnel process is gone.

## Publish only when requested

`scripts/deploy.py` requires a clean committed `master` checkout; publishing is
a separate operation from a worktree demo. Follow explicit authorization for
working on master and the global workflow; never bypass the branch check.
Install locked tooling with `npm ci`; check `npm exec -- wrangler whoami` and
use `npm exec -- wrangler login --use-keyring` if authentication is needed.
`python3 scripts/deploy.py <sample|all>` rebuilds and publishes the complete
site even for a named sample. Inspect that script and `wrangler.jsonc` for
current deployment settings. Verify its live URL checks and intended content.
R2 credentials, when needed for baseline storage, are at
`~/.config/battlement/r2.env`; do not print their contents.
