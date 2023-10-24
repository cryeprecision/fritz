use anyhow::Context;
use chrono::{Local, TimeZone};
use fritz_log_parser::{db, fritz, logger};

macro_rules! repetition {
    () => {
        None
    };
    ([$hour:literal, $minute:literal, $second:literal], $count:literal) => {
        Some(::fritz_log_parser::fritz::Repetition {
            datetime: Local
                .with_ymd_and_hms(2023, 01, 01, $hour, $minute, $second)
                .single()
                .unwrap(),
            count: $count,
        })
    };
}

macro_rules! log {
    ([$hour:literal, $minute:literal, $second:literal], $message_id:literal, $category_id:literal, $($repetition:tt)+) => {
        ::fritz_log_parser::fritz::Log {
            datetime: Local
                .with_ymd_and_hms(2023, 01, 01, $hour, $minute, $second)
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
    db: &db::Database,
    logs: &[fritz::Log],
) -> anyhow::Result<Vec<fritz::Log>> {
    for i in 0..logs.len() {
        let _ = db
            .append_new_logs(&logs[i..i + 1])
            .await
            .with_context(|| format!("insert {}", i))?;
    }
    db.select_latest_logs(0, 500).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    logger::init().context("initialize logger")?;
    let path = dotenv::dotenv().context("load .env file")?;
    log::info!("loaded .env from {}", path.to_str().expect("utf-8"));

    // let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://logs.db3".to_string());
    let db = db::Database::open_in_memory()
        .await
        .context("open database")?;

    {
        db.clear_logs().await?;

        let _ = insert_logs_single(
            &db,
            &vec![
                log!([01, 01, 01], 01, 01, repetition!([01, 01, 01], 2)),
                log!([01, 01, 01], 01, 01, repetition!([01, 01, 01], 3)),
                log!([01, 01, 02], 01, 01, repetition!([01, 01, 01], 4)),
                log!([01, 01, 03], 01, 01, repetition!([01, 01, 01], 5)),
            ],
        )
        .await?;

        db.append_new_logs(&vec![
            log!([01, 01, 03], 01, 01, repetition!([01, 01, 01], 5)),
            log!([01, 01, 04], 02, 02, repetition!()),
        ])
        .await?;

        let expected = vec![
            log!([01, 01, 04], 02, 02, repetition!()),
            log!([01, 01, 03], 01, 01, repetition!([01, 01, 01], 5)),
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
            log!([01, 01, 01], 01, 01, repetition!()),
            log!([01, 01, 01], 01, 01, repetition!([01, 01, 01], 2)),
            log!([01, 01, 02], 01, 01, repetition!([01, 01, 01], 3)),
            log!([01, 01, 03], 01, 01, repetition!([01, 01, 01], 4)),
        ];
        let expected = vec![log!([01, 01, 03], 01, 01, repetition!([01, 01, 01], 4))];

        let db_logs = insert_logs_single(&db, &logs).await?;

        log::info!("final db_logs: {:#?}", db_logs);
        if db_logs != expected {
            log::error!("lhs != rhs\n\tlhs: {:#?}\n\trhs: {:#?}", db_logs, expected)
        }
    }

    db.close().await;
    Ok(())
}
