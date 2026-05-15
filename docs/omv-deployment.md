# OpenMediaVault Deployment

This app is intended for home LAN or VPN use. Do not expose it directly to the
public internet without adding TLS, stronger auth, rate limiting, and backups.

## 1. Create an App Data Folder

On OMV, create a shared folder dedicated to this app, for example:

```text
/srv/dev-disk-by-uuid-XXXX/python-sandbox-data
```

Do not mount your media, family documents, backups, or other NAS shares into the
app. The MVP runs Python in the browser, but the app data folder should still be
treated as isolated application state.

## 2. Configure Compose

Copy `.env.example` to `.env` and set a password:

```sh
SANDBOX_USERNAME=parent
SANDBOX_PASSWORD=choose-a-real-password
SANDBOX_HTTP_PORT=8090
```

In `docker-compose.yml`, replace the local `./data` bind mount with your OMV
folder if desired:

```yaml
volumes:
  - /srv/dev-disk-by-uuid-XXXX/python-sandbox-data:/app/data
```

## 3. Start

```sh
docker compose up -d --build
```

Then open:

```text
http://YOUR-OMV-LAN-IP:8090
```

## 4. Backups

Back up the app data folder. It contains:

- `sandbox.db`: users, sessions, projects, lessons, attempts
- `projects/`: saved project files

## 5. Security Notes

- The server does not execute submitted Python in this MVP.
- Python runs inside the browser using Pyodide.
- Keep this app LAN-only unless you add production-grade hardening.
- If server-side Python execution is added later, run it in a separate sandbox
  service using gVisor or a VM, not directly on the OMV host.
