# Battlement

Battlement connects Rust game rules to a Unity rendering and input client.
Code and executable configuration are the source of truth. Read the relevant
implementation before changing it; do not load whole document collections.

## Find what you need

- `crates/`: Rust protocol, runtime, UI, tooling, and test doubles. Read
  [Rust conventions](crates/AGENTS.md) before editing Rust, including sample code.
- `Packages/com.battlement.client/`: Unity runtime, editor integration, and tests.
- `samples/`: standalone Unity projects and Rust rules; read
  [sample guidance](samples/AGENTS.md) when working there.
- `scripts/`: CI, reproduction, builds, and deployment entrypoints.
- `.agents/skills/`: load only the skill matching the task:
  `battlement-runtime` for cross-layer ownership, `battlement-reactant` for UI
  grounding, `battlement-build` for players/plugins/generated assets,
  `battlement-ditto` for native scenario checks, `battlement-ci` for validation
  and failure diagnosis, `battlement-web` for web review or deployment.
- `docs/reactant/chess-ui-implementation-plan.md`: retained chess port plan;
  read only for chess port tasks and follow its selected-page reading guide.

## Work and validation

- Use the wt skill at `~/.llms/skills/wt/SKILL.md` unless explicitly asked
  to work "on master".
  Never edit the main checkout otherwise. Continue follow-ups in this task's
  own worktree until promotion; never use another task's worktree.
- Stage all intended changes before `./scripts/ci.py`; its metadata refresh
  requires staged inputs. Run it successfully before completing work.
- Prefer black-box tests and native Ditto one-off scenarios. Use unit tests
  sparingly for complex code; do not test simple implementation details.
  Interactive web testing is only for specifically web features.
- Commit once locally and immediately submit `tg candidate HEAD` without
  promotion authority. Use Conventional Commits with a short imperative
  description; include a body only for large or non-obvious changes.
- Promotion requires explicit approval: authorize the exact candidate with
  `tg approve <candidate-id>`. Tollgate owns certified promotion and remote
  synchronization; use its configuration and the wt skill for branch targets.
  Worktree branches stay local. Never create remote branches unless requested.
- For major work (>500 non-test lines), use
  `~/.llms/skills/independent-review/SKILL.md`, verify findings,
  and fix confirmed issues. Run at most one review per session, including follow-ups.
- Web-visible features require the demo and Cloudflare Quick Tunnel lifecycle in
  [battlement-web](.agents/skills/battlement-web/SKILL.md), including cleanup
  immediately before promotion authorization.
- Do not print a summary of changes.

## Code guidelines

- Target 500 lines per source file; exceeding 1000 needs strong justification.
  Compose small parts with minimal state; follow SOLID, not partial-class workarounds.
- Backwards compatibility never matters for Masonry. Do not preserve it or
  devise versioning schemes.
- Panic for obvious developer errors with no plausible recovery; do not use `Result`.
- Avoid conditions with more than two Boolean operators.
- Document public types/functions briefly unless their names are sufficient.
  Do not repeat inherited trait, parent-class, or Unity message documentation.
- Describe current behavior, never task history, implementation stages, or plans
  in code, comments, or maintained guidance.

## Maintaining guidance

- Correct or delete incorrect guidance encountered during relevant work.
  Read code to resolve facts; preserve explicit user policies when facts change.
- Replace or remove text before expanding. Add only durable, non-obvious
  grounding or a demonstrated recurring workflow; a new feature needs no paragraph.
- Keep each rule in one home. Link to code, manifests, or command help instead
  of copying APIs, versions, component counts, inventories, or defaults.
- Keep plans, progress logs, and release evidence out of maintained guidance.
  The retained chess plan is a temporary exception, not a documentation model.
- Aim for 80 root lines, 30–60 lines per skill, and about 2500 words total across
  maintained guidance. These are editorial ceilings to guide pruning, not quotas.
