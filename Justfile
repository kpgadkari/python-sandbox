set dotenv-load := true

default:
    @just --list

# Start the full local stack at http://localhost:8090.
up:
    docker compose up --build

# Start the full local stack in the background.
up-detached:
    docker compose up -d --build

# Stop the local stack.
down:
    docker compose down

# Follow logs for all services.
logs:
    docker compose logs -f

# Start only MariaDB for backend development and integration tests.
mariadb:
    docker compose up -d mariadb

# Backward-compatible alias for the old MySQL recipe name.
mysql:
    @just mariadb

# Run the backend locally against the Compose MariaDB service.
backend-dev:
    docker compose up -d mariadb
    DATABASE_URL="${DATABASE_URL:-mysql://sandbox:${SANDBOX_MARIADB_PASSWORD:-${SANDBOX_MYSQL_PASSWORD:-sandbox}}@127.0.0.1:${SANDBOX_MARIADB_PORT:-${SANDBOX_MYSQL_PORT:-3306}}/python_sandbox}" cargo run --manifest-path backend/Cargo.toml

# Run the frontend dev server.
frontend-dev:
    npm --prefix frontend run dev

# Run backend and frontend tests.
test:
    cargo test --manifest-path backend/Cargo.toml
    npm --prefix frontend test

# Run backend tests against the Compose MariaDB service.
test-db:
    docker compose up -d mariadb
    SANDBOX_TEST_DATABASE_URL="${SANDBOX_TEST_DATABASE_URL:-mysql://sandbox:${SANDBOX_MARIADB_PASSWORD:-${SANDBOX_MYSQL_PASSWORD:-sandbox}}@127.0.0.1:${SANDBOX_MARIADB_PORT:-${SANDBOX_MYSQL_PORT:-3306}}/python_sandbox}" cargo test --manifest-path backend/Cargo.toml

# Run frontend coverage and backend tests.
coverage:
    cargo test --manifest-path backend/Cargo.toml
    npm --prefix frontend run coverage

# Build frontend assets and backend release binary.
build:
    npm --prefix frontend run build
    cargo build --release --manifest-path backend/Cargo.toml

# Build Docker images for the full stack.
docker-build:
    docker compose -f compose.yaml build

# Production-style deploy (creates .env if needed, starts stack in background).
deploy:
    ./deploy.sh
