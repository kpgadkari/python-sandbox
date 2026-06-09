CREATE TABLE submissions (
    id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    lesson_id VARCHAR(191) NOT NULL,
    code_snapshot MEDIUMTEXT NOT NULL,
    stdout MEDIUMTEXT NOT NULL,
    note TEXT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    parent_feedback TEXT NULL,
    created_at DATETIME(6) NOT NULL,
    reviewed_at DATETIME(6) NULL,
    PRIMARY KEY (id),
    CONSTRAINT fk_submissions_user
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE CASCADE,
    CONSTRAINT fk_submissions_lesson
        FOREIGN KEY (lesson_id) REFERENCES lessons(id)
        ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE INDEX idx_submissions_status ON submissions(status);
CREATE INDEX idx_submissions_user_id ON submissions(user_id);
CREATE INDEX idx_submissions_created_at ON submissions(created_at);
