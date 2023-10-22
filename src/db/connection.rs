use anyhow::Context;
use sqlx::SqlitePool;

use crate::fritz;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open_in_memory() -> anyhow::Result<Database> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .context("open sqlite in memory")?;
        Self::migrate(&pool).await?;

        Ok(Database { pool })
    }
    pub async fn open(url: &str) -> anyhow::Result<Database> {
        let pool = SqlitePool::connect(url)
            .await
            .context("connect to sqlite")?;
        Self::migrate(&pool).await?;

        Ok(Database { pool })
    }

    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations/")
            .run(pool)
            .await
            .context("migrate database")?;
        Ok(())
    }

    pub async fn append_log(&self, log: &fritz::Log) -> anyhow::Result<()> {
        let datetime = log.datetime.timestamp_millis();
        let repetition_datetime = log
            .repetition
            .as_ref()
            .map(|r| r.datetime.timestamp_millis());
        let repetition_count = log.repetition.as_ref().map(|r| r.count);

        sqlx::query!(
            r#"
        INSERT INTO "logs"
        (
            "datetime",
            "message",
            "message_id",
            "category_id",
            "repetition_datetime",
            "repetition_count"
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            datetime,
            log.message,
            log.message_id,
            log.category_id,
            repetition_datetime,
            repetition_count
        )
        .execute(&self.pool)
        .await
        .context("insert log")?;

        Ok(())
    }
    pub async fn append_logs(&self, logs: &[fritz::Log]) -> anyhow::Result<()> {
        for log in logs {
            self.append_log(log).await?;
        }
        Ok(())
    }

    pub async fn select_logs(
        &self,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<fritz::Log>> {
        let offset = i64::try_from(offset).context("cast offset as i64")?;
        let limit = i64::try_from(limit).context("cast limit as i64")?;
        sqlx::query_as!(
            super::Log,
            r#"
        SELECT "datetime",
               "message",
               "message_id",
               "category_id",
               "repetition_datetime",
               "repetition_count"
        FROM "logs"
        ORDER BY "datetime" DESC
        LIMIT ?1, ?2
            "#,
            offset,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .context("fetch logs")?
        .into_iter()
        .map(|log| log.try_into())
        .collect::<Result<Vec<_>, _>>()
    }
}
