# Python Sandbox for OMV

A home-LAN Python coding sandbox for kids. The app is hosted on an OpenMediaVault
fileserver, but Python code runs in the browser through Pyodide/WebAssembly.

## Stack

- Frontend: React, Vite, TypeScript, CodeMirror 6
- Runtime: Pyodide in a Web Worker
- Backend: Rust, Axum, SQLite
- Deployment: Docker Compose

## Local Development

Backend:

```sh
cd backend
cargo run
```

Frontend:

```sh
cd frontend
npm install
npm run dev
```

The frontend dev server proxies `/api` to `http://localhost:8080`.

Default login:

- Username: `parent`
- Password: `change-me`

Override these with `SANDBOX_USERNAME` and `SANDBOX_PASSWORD`.

## OMV Deployment

See [docs/omv-deployment.md](docs/omv-deployment.md).
