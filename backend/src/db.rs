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
            role TEXT NOT NULL DEFAULT 'parent',
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
            description TEXT NOT NULL DEFAULT '',
            hint TEXT NOT NULL DEFAULT '',
            difficulty TEXT NOT NULL DEFAULT 'Beginner',
            starter_code TEXT NOT NULL,
            expected_stdout TEXT NOT NULL,
            hidden_tests TEXT NOT NULL DEFAULT '[]',
            is_published INTEGER NOT NULL DEFAULT 1,
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
    add_column_if_missing(conn, "users", "role", "TEXT NOT NULL DEFAULT 'parent'")?;
    add_column_if_missing(conn, "lessons", "description", "TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(conn, "lessons", "hint", "TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(
        conn,
        "lessons",
        "difficulty",
        "TEXT NOT NULL DEFAULT 'Beginner'",
    )?;
    add_column_if_missing(
        conn,
        "lessons",
        "is_published",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

pub(crate) fn seed_users(conn: &Connection) -> anyhow::Result<()> {
    seed_user(
        conn,
        &env::var("SANDBOX_USERNAME").unwrap_or_else(|_| "parent".into()),
        &env::var("SANDBOX_PASSWORD").unwrap_or_else(|_| "change-me".into()),
        &env::var("SANDBOX_DISPLAY_NAME").unwrap_or_else(|_| "Parent".into()),
        "parent",
    )?;
    seed_user(
        conn,
        &env::var("SANDBOX_CHILD_USERNAME").unwrap_or_else(|_| "son".into()),
        &env::var("SANDBOX_CHILD_PASSWORD").unwrap_or_else(|_| "python".into()),
        &env::var("SANDBOX_CHILD_DISPLAY_NAME").unwrap_or_else(|_| "Young Coder".into()),
        "child",
    )
}

fn seed_user(
    conn: &Connection,
    username: &str,
    password: &str,
    display_name: &str,
    role: &str,
) -> anyhow::Result<()> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE username = ?",
            [username],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = exists {
        conn.execute(
            "UPDATE users SET display_name = ?, role = ? WHERE id = ?",
            params![display_name, role, id],
        )?;
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("hash password: {err}"))?
        .to_string();
    conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, role, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        params![Uuid::new_v4().to_string(), username, password_hash, display_name, role, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub(crate) fn seed_lessons(conn: &Connection) -> anyhow::Result<()> {
    let lessons = [
        (
            "hello-python",
            "Hello, Python",
            "Print a friendly greeting.",
            "Use print() to make Python write a message to the console.",
            "The text inside print() needs quotation marks.",
            "Basics",
            "print(\"hello, python\")\n",
            "hello, python\n",
        ),
        (
            "name-input",
            "Ask for a Name",
            "Ask for a name and print a greeting. Try running it and answering the prompt.",
            "Programs can ask questions with input(). Put Ada in the input box before you run this one.",
            "Save input() into a variable, then print it.",
            "Basics",
            "name = input(\"Name? \")\nprint(\"Hi\", name)\n",
            "Hi Ada\n",
        ),
        (
            "tiny-loop",
            "Tiny Loop",
            "Use a loop to print the numbers 1 through 3.",
            "A for loop lets Python repeat work. This lesson prints one number on each line.",
            "range(1, 4) gives you 1, 2, and 3.",
            "Loops",
            "for n in range(1, 4):\n    print(n)\n",
            "1\n2\n3\n",
        ),
        (
            "favorite-color",
            "Favorite Color",
            "Store a favorite color in a variable and print it.",
            "Variables are named boxes for values. Make a variable named color.",
            "Try: color = \"green\"",
            "Variables",
            "color = \"green\"\nprint(color)\n",
            "green\n",
        ),
        (
            "two-lines",
            "Two Lines",
            "Print cat on one line and dog on the next line.",
            "Each print() call writes a new line.",
            "Use two print() calls.",
            "Basics",
            "print(\"cat\")\nprint(\"dog\")\n",
            "cat\ndog\n",
        ),
        (
            "add-numbers",
            "Add Numbers",
            "Create two number variables and print their sum.",
            "Python can add numbers with +. Store the answer or print the expression directly.",
            "a + b adds two numbers.",
            "Variables",
            "a = 2\nb = 3\nprint(a + b)\n",
            "5\n",
        ),
        (
            "age-next-year",
            "Age Next Year",
            "Ask for an age, then print the age plus one.",
            "input() gives text. Use int() to turn the answer into a number.",
            "age = int(input(\"Age? \"))",
            "Input",
            "age = int(input(\"Age? \"))\nprint(age + 1)\n",
            "11\n",
        ),
        (
            "if-else",
            "Sunny or Rainy",
            "Use if and else to print outside when weather is sunny.",
            "Conditionals let Python choose between paths.",
            "Use == to compare two values.",
            "Conditionals",
            "weather = \"sunny\"\nif weather == \"sunny\":\n    print(\"outside\")\nelse:\n    print(\"inside\")\n",
            "outside\n",
        ),
        (
            "count-to-five",
            "Count to Five",
            "Use a loop to print numbers 1 through 5.",
            "Loops are great for counting.",
            "range(1, 6) stops before 6.",
            "Loops",
            "for n in range(1, 6):\n    print(n)\n",
            "1\n2\n3\n4\n5\n",
        ),
        (
            "list-foods",
            "Favorite Foods",
            "Make a list of three foods and print the second one.",
            "Lists keep multiple values in order. Python starts counting positions at 0.",
            "foods[1] is the second item.",
            "Lists",
            "foods = [\"pizza\", \"tacos\", \"pasta\"]\nprint(foods[1])\n",
            "tacos\n",
        ),
        (
            "make-function",
            "Make a Function",
            "Create a function that returns a greeting.",
            "Functions package code so you can reuse it.",
            "Use return to send a value back.",
            "Functions",
            "def greeting(name):\n    return \"Hi \" + name\n\nprint(greeting(\"Ada\"))\n",
            "Hi Ada\n",
        ),
        (
            "mini-quiz",
            "Mini Quiz",
            "Ask a question and print correct if the answer is python.",
            "This combines input, variables, and if statements.",
            "Compare the answer with \"python\".",
            "Mini Project",
            "answer = input(\"Best language? \")\nif answer == \"python\":\n    print(\"correct\")\nelse:\n    print(\"try again\")\n",
            "correct\n",
        ),
    ];

    for (index, lesson) in lessons.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO lessons (id, title, prompt, description, hint, difficulty, starter_code, expected_stdout, hidden_tests, is_published, sort_order)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', 1, ?)",
            params![lesson.0, lesson.1, lesson.2, lesson.3, lesson.4, lesson.5, lesson.6, lesson.7, index as i64],
        )?;
        conn.execute(
            "UPDATE lessons SET title = ?, prompt = ?, description = ?, hint = ?, difficulty = ?, starter_code = ?, expected_stdout = ?, sort_order = ?, is_published = 1 WHERE id = ?",
            params![lesson.1, lesson.2, lesson.3, lesson.4, lesson.5, lesson.6, lesson.7, index as i64, lesson.0],
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
        assert_eq!(count, 12);
        assert_eq!(first_title, "Hello, Python");
        Ok(())
    }
}
