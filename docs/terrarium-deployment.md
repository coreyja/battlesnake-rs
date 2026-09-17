# Terrarium deployment

GitHub's `Deploy Terrarium` workflow builds the `main` revision, connects through
Tailscale, installs the binary as `www`, and restarts
`terrarium.coreyja.com.service` as `circle`. On Terrarium, `circle` has passwordless
sudo; the service runs as `www` from `/home/www/server`.

The workflow needs the existing `DEPLOY_SSH_KEY`, `TS_OAUTH_CLIENT_ID`, and
`TS_OAUTH_AUDIENCE` secrets, plus these Jev/Eyes settings:

| Kind | Name | Purpose |
| --- | --- | --- |
| Secret | `TYPESAFE_API_KEY` | TypeSafe API credential |
| Secret | `EYES_TOKEN` | Dedicated Judicious Jev app's ingest token |
| Variable | `EYES_ORG_ID` | Eyes organization UUID |
| Variable | `EYES_APP_ID` | Judicious Jev app UUID |

Before replacing the binary, deployment validates that all four values are present
and contain no newline or NUL characters. It transfers them over SSH into a mode
`0600` file owned by `www`, `/home/www/server/judicious-jev.env`, using a temporary
file and rename. Credential values do not appear in logs or command arguments.

Deployment then installs a root-owned systemd drop-in at
`/etc/systemd/system/terrarium.coreyja.com.service.d/judicious-jev.conf`, alongside
the existing service configuration, and reloads systemd. The drop-in requires the
environment file. This setup is repeated on deployment, so no manual console step
is needed. Update GitHub's settings and redeploy to rotate credentials.

After restart, the workflow checks the running process for the four nonempty
settings without printing values, and checks the public homepage for HTTP success.

The workflow's manual `inspect` operation reports service status, environment-file
paths, deployment-file ownership, and sudo permissions without modifying the host
or deploying a binary. Run it from `main`: Tailscale accepted `main` while rejecting
the Jev feature branch's federated login with HTTP 403.

Tailscale SSH is separate from key-based SSH over Tailscale. Enabling it on the
droplet requires an SSH policy rule allowing `tag:ci` to connect as `www` and
`circle`, or it will intercept and reject the existing deployment connection.
