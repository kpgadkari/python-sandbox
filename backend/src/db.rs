use std::env;

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use rand::rngs::OsRng;
use sqlx::MySqlPool;
use uuid::Uuid;

pub(crate) async fn seed_users(pool: &MySqlPool) -> anyhow::Result<()> {
    seed_user(
        pool,
        &env::var("SANDBOX_USERNAME").unwrap_or_else(|_| "parent".into()),
        &env::var("SANDBOX_PASSWORD").unwrap_or_else(|_| "change-me".into()),
        &env::var("SANDBOX_DISPLAY_NAME").unwrap_or_else(|_| "Parent".into()),
        "parent",
    )
    .await?;
    seed_user(
        pool,
        &env::var("SANDBOX_CHILD_USERNAME").unwrap_or_else(|_| "son".into()),
        &env::var("SANDBOX_CHILD_PASSWORD").unwrap_or_else(|_| "python".into()),
        &env::var("SANDBOX_CHILD_DISPLAY_NAME").unwrap_or_else(|_| "Young Coder".into()),
        "child",
    )
    .await
}

async fn seed_user(
    pool: &MySqlPool,
    username: &str,
    password: &str,
    display_name: &str,
    role: &str,
) -> anyhow::Result<()> {
    let exists = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    if let Some(id) = exists {
        sqlx::query("UPDATE users SET display_name = ?, role = ? WHERE id = ?")
            .bind(display_name)
            .bind(role)
            .bind(id)
            .execute(pool)
            .await?;
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("hash password: {err}"))?
        .to_string();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, role, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(role)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn seed_lessons(pool: &MySqlPool) -> anyhow::Result<()> {
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
        sqlx::query(
            "INSERT INTO lessons (
                id, title, prompt, description, hint, difficulty, starter_code,
                expected_stdout, hidden_tests, is_published, sort_order
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', TRUE, ?)
             ON DUPLICATE KEY UPDATE
                title = VALUES(title),
                prompt = VALUES(prompt),
                description = VALUES(description),
                hint = VALUES(hint),
                difficulty = VALUES(difficulty),
                starter_code = VALUES(starter_code),
                expected_stdout = VALUES(expected_stdout),
                sort_order = VALUES(sort_order),
                is_published = TRUE",
        )
        .bind(lesson.0)
        .bind(lesson.1)
        .bind(lesson.2)
        .bind(lesson.3)
        .bind(lesson.4)
        .bind(lesson.5)
        .bind(lesson.6)
        .bind(lesson.7)
        .bind(index as i32)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seeds_default_lessons_once() -> anyhow::Result<()> {
        let database_url = match env::var("SANDBOX_TEST_DATABASE_URL") {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };
        let pool = MySqlPool::connect(&database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        sqlx::query("DELETE FROM attempts").execute(&pool).await?;
        sqlx::query("DELETE FROM lessons").execute(&pool).await?;

        seed_lessons(&pool).await?;
        seed_lessons(&pool).await?;

        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM lessons")
            .fetch_one(&pool)
            .await?;
        let first_title = sqlx::query_scalar::<_, String>(
            "SELECT title FROM lessons ORDER BY sort_order ASC LIMIT 1",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(count, 12);
        assert_eq!(first_title, "Hello, Python");
        Ok(())
    }
}
