# Python Sandbox for OMV

A home-LAN Python coding sandbox for kids. The app is hosted on an OpenMediaVault
fileserver, but Python code runs in the browser through Pyodide/WebAssembly.

## Stack

- Frontend: React, Vite, TypeScript, CodeMirror 6
- Runtime: Pyodide in a Web Worker
- Backend: Rust, Axum, MariaDB (MySQL-compatible protocol)
- Deployment: Docker Compose

## Local Development

Install [`just`](https://just.systems/) to run common tasks from the repository root:

```sh
brew install just
just --list
```

Copy `.env.example` to `.env` and adjust passwords/ports as needed before
running the Docker stack.

Backend:

```sh
export DATABASE_URL=mysql://sandbox:sandbox@127.0.0.1:3306/python_sandbox
just backend-dev
```

Frontend:

```sh
npm install
just frontend-dev
```

The frontend dev server proxies `/api` to `http://localhost:8080`.

Useful recipes:

```sh
just test          # frontend + backend tests
just coverage      # frontend + backend coverage checks
just build         # frontend bundle + backend release binary
just docker-build  # Docker images for both services
just up            # start the Docker Compose stack
```

Default login:

- Username: `parent`
- Password: `change-me`

Override these with `SANDBOX_USERNAME` and `SANDBOX_PASSWORD`.

The Docker Compose stack includes MariaDB. For local backend-only development,
start the database with `just mariadb` or point `DATABASE_URL` at another
MariaDB/MySQL-compatible server. Compose publishes MariaDB on
`127.0.0.1:${SANDBOX_MARIADB_PORT:-3306}` for local development and integration
tests.

Set `SANDBOX_TEST_DATABASE_URL` to run backend database integration tests against
a disposable MariaDB database.

## Deploy (OMV or any Docker host)

```sh
cp .env.example .env   # set passwords and SANDBOX_DATA_PATH on OMV
./deploy.sh
```

Starts MariaDB, backend, and frontend from a single `compose.yaml`. See
[docs/omv-deployment.md](docs/omv-deployment.md).
