CREATE TABLE users (
    id CHAR(36) PRIMARY KEY,
    username VARCHAR(191) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(191) NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'parent',
    created_at DATETIME(6) NOT NULL
);

CREATE TABLE sessions (
    token CHAR(64) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    created_at DATETIME(6) NOT NULL,
    expires_at DATETIME(6) NOT NULL,
    CONSTRAINT fk_sessions_user
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

CREATE TABLE projects (
    id CHAR(36) PRIMARY KEY,
    owner_id CHAR(36) NOT NULL,
    title VARCHAR(255) NOT NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT fk_projects_owner
        FOREIGN KEY (owner_id) REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_projects_owner_id ON projects(owner_id);
CREATE INDEX idx_projects_updated_at ON projects(updated_at);

CREATE TABLE lessons (
    id VARCHAR(191) PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    prompt TEXT NOT NULL,
    description TEXT NOT NULL,
    hint TEXT NOT NULL,
    difficulty VARCHAR(64) NOT NULL DEFAULT 'Beginner',
    starter_code MEDIUMTEXT NOT NULL,
    expected_stdout MEDIUMTEXT NOT NULL,
    hidden_tests MEDIUMTEXT NOT NULL,
    is_published BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order INT NOT NULL
);

CREATE TABLE attempts (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    lesson_id VARCHAR(191) NOT NULL,
    code_snapshot MEDIUMTEXT NOT NULL,
    stdout MEDIUMTEXT NOT NULL,
    passed BOOLEAN NOT NULL,
    created_at DATETIME(6) NOT NULL,
    CONSTRAINT fk_attempts_user
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE CASCADE,
    CONSTRAINT fk_attempts_lesson
        FOREIGN KEY (lesson_id) REFERENCES lessons(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_attempts_user_id ON attempts(user_id);
CREATE INDEX idx_attempts_lesson_id ON attempts(lesson_id);
