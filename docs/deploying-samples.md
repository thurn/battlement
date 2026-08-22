# Deploying the sample site

Battlement's Unity Web samples are hosted as static assets at
`https://samples.battlement.workers.dev/`. The root page links to each sample at
its own path, including `/basic/`, `/chess/`, and `/tictactoe/`.

## Prerequisites

Install Unity 6000.5.8f1, Rust, Node.js, and npm. From the repository root,
install the pinned Cloudflare Wrangler version and authenticate it:

```sh
npm ci
npm exec -- wrangler login --use-keyring
npm exec -- wrangler whoami
```

The authenticated Cloudflare account must own the `battlement.workers.dev`
subdomain. If Wrangler lists multiple accounts, set `CLOUDFLARE_ACCOUNT_ID` to
the intended account ID for the deployment command.

## Deploy

Deploy from a clean, committed `master` checkout:

```sh
python3 scripts/deploy.py all
```

A sample name is also accepted:

```sh
python3 scripts/deploy.py chess
```

Cloudflare static-asset versions contain the complete site, so a named command
builds that sample first and then rebuilds the other samples before publishing.
No Git integration or automatic deployment is configured.

The script builds threaded release WebGL players, stages the ignored site under
`Build/cloudflare`, validates Cloudflare's asset limits and Unity compression,
deploys the `samples` Worker, and checks the live sample URLs. Build or
validation failures leave the current deployment untouched.

## Inspect or roll back

Open **Workers & Pages > samples** in the Cloudflare dashboard to inspect
deployments. Wrangler can show recent versions and roll back the latest deploy:

```sh
npm exec -- wrangler deployments list --name samples
npm exec -- wrangler rollback --name samples
```

Cloudflare offers convenient rollback to the 100 most recently published
versions. At 100 deployments per day, that is approximately one day of history.
A custom domain can be attached later without changing the sample paths.
