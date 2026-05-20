# OpenMediaVault Deployment

One-command deploy on the NAS (or any Docker host with Compose):

```sh
git clone <repo-url> python-sandbox
cd python-sandbox
cp .env.example .env
# Edit .env: passwords and SANDBOX_DATA_PATH (OMV shared folder)
./deploy.sh
```

Open `http://<OMV-LAN-IP>:8090` (or the URL printed by `deploy.sh`).

This app is intended for home LAN or VPN use. Do not expose it directly to the
public internet without adding TLS, stronger auth, rate limiting, and backups.

## What `deploy.sh` does

- Creates `.env` from `.env.example` if missing
- Creates the app data directory (`SANDBOX_DATA_PATH`)
- Runs `docker compose -f compose.yaml up -d --build`
- Starts **MariaDB**, **backend**, and **frontend** with a one-shot **data-init**
  container that fixes permissions on the data folder (UID 10001)

## OMV shared folder

1. In OMV: **Storage → Shared Folders** → create `python-sandbox-data`.
2. In `.env` set the full path:

```sh
SANDBOX_DATA_PATH=/srv/dev-disk-by-uuid-XXXX/python-sandbox-data
SANDBOX_PASSWORD=choose-a-real-password
SANDBOX_MARIADB_PASSWORD=choose-a-database-password
SANDBOX_MARIADB_ROOT_PASSWORD=choose-a-root-database-password
```

Do not mount media libraries or backups into the app—only this dedicated folder.

## OMV Compose plugin

1. Copy or clone this repository onto the NAS.
2. Add a stack that points at `compose.yaml` in the repo root.
3. Set the same variables from `.env` in the stack environment (or upload `.env`).
4. Deploy the stack.

Alternatively, SSH in and run `./deploy.sh` from the repo directory.

## Manage the stack

```sh
docker compose -f compose.yaml logs -f
docker compose -f compose.yaml ps
docker compose -f compose.yaml down
docker compose -f compose.yaml up -d --build   # rebuild after updates
```

## Backups

Back up both persistent locations:

- Docker volume `mariadb-data` (or `python-sandbox_mariadb-data`): users, sessions,
  projects metadata, lessons, attempts
- `SANDBOX_DATA_PATH/projects/`: saved project files on disk

```sh
docker compose -f compose.yaml exec mariadb \
  mariadb-dump -u sandbox -p"${SANDBOX_MARIADB_PASSWORD}" python_sandbox > backup.sql
```

## External MariaDB

If MariaDB already runs on OMV, remove the `mariadb` service from `compose.yaml`,
point `DATABASE_URL` at your instance, and drop the `mariadb` dependency from
`backend`.

## Security notes

- Python runs in the browser (Pyodide); the server does not execute submitted code.
- Keep the app LAN-only unless you add a reverse proxy, TLS, and hardening.
- Use strong values for `SANDBOX_PASSWORD` and `SANDBOX_MARIADB_*`.
