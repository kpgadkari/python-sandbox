# AGENTS.md

## Cursor Cloud specific instructions

### Architecture overview

Python Sandbox for OMV: a home-LAN Python coding sandbox with a **Rust/Axum backend** (`backend/`), **React/Vite/TypeScript frontend** (`frontend/`), and **MariaDB** (via Docker Compose). Python code runs client-side via Pyodide/WebAssembly — no server-side Python runtime needed.

### Prerequisites

- **Rust** ≥ 1.85 (edition 2024 required by transitive dependency `home`). Run `rustup default stable` if the default toolchain is too old.
- **Node.js** (v18+) and **npm** (lockfile: `frontend/package-lock.json`).
- **Docker** and **Docker Compose** (for MariaDB).
- **`just`** command runner (recipes in `Justfile`).

### Running services

All standard commands are documented in the README. Key notes:

1. **MariaDB**: `docker compose up -d mariadb` (or `just mariadb`). Wait for healthcheck before starting the backend.
2. **Backend**: `just backend-dev` starts MariaDB and the Rust API server on port 8080. Requires `DATABASE_URL` env var (defaults to `mysql://sandbox:sandbox@127.0.0.1:3306/python_sandbox`). Runs migrations and seeds users/lessons on startup.
3. **Frontend**: `npm --prefix frontend run dev` (or `just frontend-dev`) starts Vite on port 5173, proxying `/api` to `http://localhost:8080`.

Copy `.env.example` to `.env` before starting. Create `data/projects/` directory if it doesn't exist.

### Testing

- `just test` — runs both frontend (vitest) and backend (cargo test) unit tests.
- `just test-db` — runs backend tests against the Compose MariaDB instance.
- `just coverage` — frontend coverage + backend tests.
- Frontend tests: `npm --prefix frontend test`
- Backend tests: `cargo test --manifest-path backend/Cargo.toml`
- TypeScript check: `npx tsc --noEmit` (in `frontend/`)
- Rust check: `cargo check --manifest-path backend/Cargo.toml`

### Default credentials

- Parent: `parent` / `change-me`
- Child: `son` / `python`
- Override via `SANDBOX_USERNAME`, `SANDBOX_PASSWORD`, `SANDBOX_CHILD_USERNAME`, `SANDBOX_CHILD_PASSWORD` env vars.

### Gotchas

- The Docker daemon must be running before `docker compose` commands work. In Cloud Agent VMs, you may need to start dockerd manually and fix socket permissions.
- Pyodide (in-browser Python) loads from `/public/pyodide/` via dynamic import in a Web Worker. In Vite dev mode, this can produce warnings about public directory imports; it works in production builds.
- Backend uses `sqlx::migrate!("./migrations")` with compile-time path — the working directory matters when building.
- API routes are at `/api/login`, `/api/me`, `/api/projects`, `/api/lessons`, etc. (no `/auth/` prefix).
