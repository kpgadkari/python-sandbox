use std::env;

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use rand::rngs::OsRng;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub(crate) fn init_db(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            display_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS lessons (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            prompt TEXT NOT NULL,
            starter_code TEXT NOT NULL,
            expected_stdout TEXT NOT NULL,
            hidden_tests TEXT NOT NULL DEFAULT '[]',
            sort_order INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS attempts (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            lesson_id TEXT NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
            code_snapshot TEXT NOT NULL,
            stdout TEXT NOT NULL,
            passed INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

pub(crate) fn seed_user(conn: &Connection) -> anyhow::Result<()> {
    let username = env::var("SANDBOX_USERNAME").unwrap_or_else(|_| "parent".into());
    let password = env::var("SANDBOX_PASSWORD").unwrap_or_else(|_| "change-me".into());
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE username = ?",
            [&username],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_some() {
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("hash password: {err}"))?
        .to_string();
    conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, created_at) VALUES (?, ?, ?, ?, ?)",
        params![Uuid::new_v4().to_string(), username, password_hash, "Home Coder", Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub(crate) fn seed_lessons(conn: &Connection) -> anyhow::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM lessons", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let lessons = [
        (
            "hello-python",
            "Hello, Python",
            "Print a friendly greeting.",
            "print(\"hello, python\")\n",
            "hello, python\n",
        ),
        (
            "name-input",
            "Ask for a Name",
            "Ask for a name and print a greeting. Try running it and answering the prompt.",
            "name = input(\"Name? \")\nprint(\"Hi\", name)\n",
            "Hi Ada\n",
        ),
        (
            "tiny-loop",
            "Tiny Loop",
            "Use a loop to print the numbers 1 through 3.",
            "for n in range(1, 4):\n    print(n)\n",
            "1\n2\n3\n",
        ),
    ];

    for (index, lesson) in lessons.iter().enumerate() {
        conn.execute(
            "INSERT INTO lessons (id, title, prompt, starter_code, expected_stdout, hidden_tests, sort_order)
             VALUES (?, ?, ?, ?, ?, '[]', ?)",
            params![lesson.0, lesson.1, lesson.2, lesson.3, lesson.4, index as i64],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_default_lessons_once() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_db(&conn)?;

        seed_lessons(&conn)?;
        seed_lessons(&conn)?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM lessons", [], |row| row.get(0))?;
        let first_title: String = conn.query_row(
            "SELECT title FROM lessons ORDER BY sort_order ASC LIMIT 1",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(count, 3);
        assert_eq!(first_title, "Hello, Python");
        Ok(())
    }
}
