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
    let username = username.trim().to_lowercase();
    let exists = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?")
        .bind(&username)
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
    .bind(Utc::now().naive_utc())
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
            "repeat-ha",
            "Laugh Loop",
            "Print ha three times on one line using *.",
            "In Python, \"ha\" * 3 becomes hahaha.",
            "Try: print(\"ha\" * 3)",
            "Basics",
            "print(\"ha\" * 3)\n",
            "hahaha\n",
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
            "multiply-numbers",
            "Multiply Numbers",
            "Store two numbers in variables and print their product.",
            "Use * to multiply numbers, just like on paper.",
            "a * b multiplies two numbers.",
            "Variables",
            "a = 4\nb = 5\nprint(a * b)\n",
            "20\n",
        ),
        (
            "my-age",
            "Age in Five Years",
            "Store your age in a variable and print the age plus five.",
            "Variables can hold numbers too. Change the age if you want.",
            "Try: age = 10 then print(age + 5)",
            "Variables",
            "age = 10\nprint(age + 5)\n",
            "15\n",
        ),
        (
            "hello-name",
            "Hello by Name",
            "Store a name in a variable and print a greeting with +.",
            "Strings can be joined with +.",
            "print(\"Hello \" + name)",
            "Strings",
            "name = \"Ada\"\nprint(\"Hello \" + name)\n",
            "Hello Ada\n",
        ),
        (
            "uppercase-shout",
            "Shout It",
            "Store a word in a variable and print it in ALL CAPS.",
            ".upper() makes letters uppercase.",
            "print(word.upper())",
            "Strings",
            "word = \"quiet\"\nprint(word.upper())\n",
            "QUIET\n",
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
            "countdown",
            "Countdown",
            "Print 3, then 2, then 1 on separate lines.",
            "range(3, 0, -1) counts backward.",
            "for n in range(3, 0, -1):",
            "Loops",
            "for n in range(3, 0, -1):\n    print(n)\n",
            "3\n2\n1\n",
        ),
        (
            "print-stars",
            "Star Line",
            "Print five stars on one line.",
            "You can repeat text with * just like ha in an earlier lesson.",
            "print(\"*\" * 5)",
            "Loops",
            "print(\"*\" * 5)\n",
            "*****\n",
        ),
        (
            "sum-one-to-three",
            "Add With a Loop",
            "Use a loop to add 1 + 2 + 3, then print the total.",
            "Start total at 0 and add each number inside the loop.",
            "total += n adds n to total.",
            "Loops",
            "total = 0\nfor n in range(1, 4):\n    total += n\nprint(total)\n",
            "6\n",
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
            "is-even",
            "Even or Odd",
            "Check if a number is even and print even or odd.",
            "The % operator gives the remainder after division. Even numbers have remainder 0 when divided by 2.",
            "if number % 2 == 0:",
            "Conditionals",
            "number = 4\nif number % 2 == 0:\n    print(\"even\")\nelse:\n    print(\"odd\")\n",
            "even\n",
        ),
        (
            "pick-grade",
            "Report Card",
            "Use if and elif to print a letter grade for a score.",
            "Check bigger scores first: 90 or more is A, 80 or more is B, otherwise C.",
            "elif means \"else if\".",
            "Conditionals",
            "score = 85\nif score >= 90:\n    print(\"A\")\nelif score >= 80:\n    print(\"B\")\nelse:\n    print(\"C\")\n",
            "B\n",
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
            "list-length",
            "How Many Foods",
            "Make a list of three foods and print how many items are in the list.",
            "len() tells you how many items a list has.",
            "print(len(foods))",
            "Lists",
            "foods = [\"pizza\", \"tacos\", \"pasta\"]\nprint(len(foods))\n",
            "3\n",
        ),
        (
            "loop-foods",
            "Food Menu",
            "Print each food in a list on its own line.",
            "for food in foods: visits every item in the list.",
            "Indent the print line inside the loop.",
            "Lists",
            "foods = [\"pizza\", \"tacos\", \"pasta\"]\nfor food in foods:\n    print(food)\n",
            "pizza\ntacos\npasta\n",
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
            "double-it",
            "Double It",
            "Write a function that doubles a number and print the result of double(6).",
            "Functions can do math and return the answer.",
            "return n * 2",
            "Functions",
            "def double(n):\n    return n * 2\n\nprint(double(6))\n",
            "12\n",
        ),
        (
            "mad-libs",
            "Tiny Mad Lib",
            "Ask for an animal and a color, then print a silly sentence.",
            "Use two input() calls. Put cat and blue in the input box (one answer per line) before you run.",
            "print(\"I saw a\", color, animal)",
            "Mini Project",
            "animal = input(\"Animal? \")\ncolor = input(\"Color? \")\nprint(\"I saw a\", color, animal)\n",
            "I saw a blue cat\n",
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
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', 1, ?)
             ON DUPLICATE KEY UPDATE
                title = ?,
                prompt = ?,
                description = ?,
                hint = ?,
                difficulty = ?,
                starter_code = ?,
                expected_stdout = ?,
                sort_order = ?,
                is_published = 1",
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
    use crate::test_db::TestDb;

    #[tokio::test]
    async fn seeds_default_lessons_once() -> anyhow::Result<()> {
        let Some(db) = TestDb::connect().await? else {
            return Ok(());
        };

        seed_lessons(&db.pool).await?;
        seed_lessons(&db.pool).await?;

        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM lessons")
            .fetch_one(&db.pool)
            .await?;
        let first_title = sqlx::query_scalar::<_, String>(
            "SELECT title FROM lessons ORDER BY sort_order ASC LIMIT 1",
        )
        .fetch_one(&db.pool)
        .await?;
        assert_eq!(count, 26);
        assert_eq!(first_title, "Hello, Python");
        Ok(())
    }
}
