---
name: battlement-web
description: Prepare a Battlement web review demo and Cloudflare Quick Tunnel, verify web-specific behavior, or deploy the sample site when publishing is requested.
---

# Web review and publishing

Shared Rust/Unity behavior uses native Ditto evidence. Use a real browser for
browser defects, JavaScript integration, WebGL rendering risks, and hosting.
`scripts/web_selection.py` selects required local contracts during aggregate CI.
For a platform risk not covered by existing paths, declare the durable risk and
its affected samples in `web/contracts.toml`; do not substitute a native pass.

Stage build inputs, then run `python3 scripts/prepare-web-demo.py <sample>`
(add `--release` when needed). Use its returned directory with
`python3 scripts/serve_web.py --directory <build-directory> --port <port>`.
Choose a verified-free port and record the process, worktree, port, and logs.
Exercise the affected behavior, inspect console and failed requests through
asset loading, and verify the rendered result. A loader or canvas is insufficient.

Create a public demo only when explicitly requested. Follow the global wt
skill's durable-service procedure for the local server and a separate service
running `cloudflared tunnel --no-autoupdate --url http://127.0.0.1:<port>`.
Record both service labels and verify the requested interaction at the local
and public URLs. Keep tunnel failures distinct from native validation.

Use only the globally configured Playwright MCP for browser automation. It is
an isolated-context client of the singleton at `http://localhost:8931/mcp`.
If unavailable, run `playwright-mcp-service start` and retry. Never directly
launch a browser/automation CLI or request a shared context. Prefer snapshots,
DOM state, and locators; take screenshots when appearance is relevant.
Close this task's browser context after QA, leaving the shared service running.

For a requested public demo, include the verified public URL and interaction
walkthrough in the handoff. Keep its durable services running through review.
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
current deployment settings. Deployment checks every shipped sample's declared
interaction on the complete local site before publishing, then checks the live
hosting response. Browser failure blocks publication; it does not add an
unconditional Web build to native candidate validation. Review evidence lives
outside the worktree in the Battlement Web compatibility cache.
R2 credentials, when needed for baseline storage, are at
`~/.config/battlement/r2.env`; do not print their contents.
