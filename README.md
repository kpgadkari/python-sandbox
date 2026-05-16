# Python Sandbox for OMV

A home-LAN Python coding sandbox for kids. The app is hosted on an OpenMediaVault
fileserver, but Python code runs in the browser through Pyodide/WebAssembly.

## Stack

- Frontend: React, Vite, TypeScript, CodeMirror 6
- Runtime: Pyodide in a Web Worker
- Backend: Rust, Axum, SQLite
- Deployment: Docker Compose

## Local Development

Install [`just`](https://just.systems/) to run common tasks from the repository root:

```sh
brew install just
just --list
```

Backend:

```sh
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

## OMV Deployment

See [docs/omv-deployment.md](docs/omv-deployment.md).
