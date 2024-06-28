use anyhow::Context;
use chrono::{Local, TimeZone};

macro_rules! repetition {
    () => {
        None
    };
    ([$hour:literal, $minute:literal, $second:literal], $count:literal) => {
        Some(crate::fritz::Repetition {
            datetime: Local
                .with_ymd_and_hms(2023, 1, 1, $hour, $minute, $second)
                .single()
                .unwrap(),
            count: $count,
        })
    };
}

macro_rules! log {
    ([$hour:literal, $minute:literal, $second:literal], $message_id:literal, $category_id:literal, $($repetition:tt)+) => {
        crate::fritz::Log {
            datetime: Local
                .with_ymd_and_hms(2023, 1, 1, $hour, $minute, $second)
                .single()
                .unwrap(),
            message: "message".to_string(),
            message_id: $message_id,
            category_id: $category_id,
            repetition: $($repetition)+,
        }
    };
}

async fn insert_logs_single(
    db: &crate::db::Database,
    logs: &[crate::fritz::Log],
) -> anyhow::Result<Vec<crate::fritz::Log>> {
    for i in 0..logs.len() {
        let _ = db
            .append_new_logs(&logs[i..i + 1])
            .await
            .with_context(|| format!("insert {}", i))?;
    }
    db.select_latest_logs(0, 500).await
}

#[tokio::test(flavor = "current_thread")]
async fn insert_new() -> anyhow::Result<()> {
    crate::log::init().context("initialize logger")?;
    let path = dotenv::dotenv().context("load .env file")?;
    log::info!("loaded .env from {}", path.to_str().expect("utf-8"));

    // let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://logs.db3".to_string());
    let db = crate::db::Database::open_in_memory()
        .await
        .context("open database")?;

    {
        db.clear_logs().await?;

        let _ = insert_logs_single(
            &db,
            &vec![
                log!([1, 1, 1], 1, 1, repetition!([1, 1, 1], 2)),
                log!([1, 1, 1], 1, 1, repetition!([1, 1, 1], 3)),
                log!([1, 1, 2], 1, 1, repetition!([1, 1, 1], 4)),
                log!([1, 1, 3], 1, 1, repetition!([1, 1, 1], 5)),
            ],
        )
        .await?;

        db.append_new_logs(&[
            log!([1, 1, 3], 1, 1, repetition!([1, 1, 1], 5)),
            log!([1, 1, 4], 2, 2, repetition!()),
        ])
        .await?;

        let expected = vec![
            log!([1, 1, 4], 2, 2, repetition!()),
            log!([1, 1, 3], 1, 1, repetition!([1, 1, 1], 5)),
        ];

        let db_logs = db.select_latest_logs(0, 500).await?;

        log::info!("final db_logs: {:#?}", db_logs);
        if db_logs != expected {
            log::error!("lhs != rhs\n\tlhs: {:#?}\n\trhs: {:#?}", db_logs, expected)
        }
    }
    {
        db.clear_logs().await?;

        let logs = vec![
            log!([1, 1, 1], 1, 1, repetition!()),
            log!([1, 1, 1], 1, 1, repetition!([1, 1, 1], 2)),
            log!([1, 1, 2], 1, 1, repetition!([1, 1, 1], 3)),
            log!([1, 1, 3], 1, 1, repetition!([1, 1, 1], 4)),
        ];
        let expected = vec![log!([1, 1, 3], 1, 1, repetition!([1, 1, 1], 4))];

        let db_logs = insert_logs_single(&db, &logs).await?;

        log::info!("final db_logs: {:#?}", db_logs);
        if db_logs != expected {
            log::error!("lhs != rhs\n\tlhs: {:#?}\n\trhs: {:#?}", db_logs, expected)
        }
    }

    db.close().await;
    Ok(())
}
