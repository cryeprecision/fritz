use anyhow::Context;
use sqlx::SqlitePool;

use super::model::{Request, Update};
use crate::fritz;

#[derive(Clone)]
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
        sqlx::migrate!("./data/migrations/")
            .run(pool)
            .await
            .context("migrate database")?;
        Ok(())
    }

    pub async fn clear_logs(&self) -> anyhow::Result<()> {
        sqlx::query!(r#"DELETE FROM "logs""#)
            .execute(&self.pool)
            .await
            .context("clear logs")?;
        Ok(())
    }

    /// Appends a log to the database without checking for consistency
    pub async fn append_log(&self, log: &fritz::Log) -> anyhow::Result<()> {
        let log = super::Log::from(log.clone());

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
            /* 1 */ log.datetime,
            /* 2 */ log.message,
            /* 3 */ log.message_id,
            /* 4 */ log.category_id,
            /* 5 */ log.repetition_datetime,
            /* 6 */ log.repetition_count
        )
        .execute(&self.pool)
        .await
        .context("insert log")?;

        Ok(())
    }

    /// Append logs to the database without checking for consistency
    pub async fn append_logs(&self, logs: &[fritz::Log]) -> anyhow::Result<()> {
        for log in logs {
            self.append_log(log).await?;
        }
        Ok(())
    }

    pub async fn logs_count(&self) -> anyhow::Result<usize> {
        let count = sqlx::query!(
            r#"
        SELECT count(*) as "count"
        FROM "logs"
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("fetch row count")?
        .count;

        usize::try_from(count).context("negative row count")
    }

    pub async fn is_empty(&self) -> anyhow::Result<bool> {
        // https://dba.stackexchange.com/a/223286
        let count = sqlx::query!(
            r#"
        SELECT count(*) as count
        FROM (SELECT 0 from logs LIMIT 1)
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("check database empty")?
        .count;

        Ok(count == 0)
    }

    /// Select the `limit` latest logs offset by `offset`.
    pub async fn select_latest_logs(
        &self,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<fritz::Log>> {
        let offset = i64::try_from(offset).context("cast offset as i64")?;
        let limit = i64::try_from(limit).context("cast limit as i64")?;
        sqlx::query_as!(
            super::Log,
            r#"
        SELECT "id",
               "datetime",
               "message",
               "message_id",
               "category_id",
               "repetition_datetime",
               "repetition_count"
        FROM "logs"
        ORDER BY "id" DESC
        LIMIT ?1, ?2
            "#,
            /* 1 */ offset,
            /* 2 */ limit,
        )
        .fetch_all(&self.pool)
        .await
        .context("fetch logs")?
        .into_iter()
        .map(|log| log.try_into())
        .collect::<Result<Vec<_>, _>>()
    }

    pub async fn select_latest_log(&self) -> anyhow::Result<Option<fritz::Log>> {
        Ok(self
            .select_latest_logs(0, 1)
            .await
            .context("select latest log")?
            .into_iter()
            .next())
    }

    pub async fn replace_log(&self, old: &fritz::Log, new: &fritz::Log) -> anyhow::Result<()> {
        let old_log = super::Log::from(old.clone());
        let new_log = super::Log::from(new.clone());

        let rows_affected = sqlx::query!(
            r#"
        UPDATE "logs"
        SET "datetime"            = ?1,
            "message"             = ?2,
            "message_id"          = ?3,
            "category_id"         = ?4,
            "repetition_datetime" = ?5,
            "repetition_count"    = ?6
        WHERE "datetime"    = ?7 AND
              "message_id"  = ?8 AND
              "category_id" = ?9
            "#,
            /* 1 */ new_log.datetime,
            /* 2 */ new_log.message,
            /* 3 */ new_log.message_id,
            /* 4 */ new_log.category_id,
            /* 5 */ new_log.repetition_datetime,
            /* 6 */ new_log.repetition_count,
            /* 7 */ old_log.datetime,
            /* 8 */ old_log.message_id,
            /* 9 */ old_log.category_id,
        )
        .execute(&self.pool)
        .await
        .context("update log")?
        .rows_affected();

        if rows_affected != 1 {
            log::error!(
                "invalid number of rows affected (got {}, expected 1)",
                rows_affected
            );
        }

        Ok(())
    }

    /// Appends the given logs to the database.
    ///
    /// Logs must be sorted from **old to new** so the oldest log is at index 0.
    ///
    /// Returns a slice over the inserted or updated elements.
    pub async fn append_new_logs<'a>(
        &self,
        logs: &'a [fritz::Log],
    ) -> anyhow::Result<&'a [fritz::Log]> {
        // Database: [3,2,1]
        //
        // [4,5]   -> [5,4,3,2,1]: All logs are new
        // [1,2]   ->     [3,2,1]: All logs are old
        // [2,3,4] ->   [4,3,2,1]: Some logs are new

        // make sure the logs are sorted from old to new
        if !logs.windows(2).all(|w| w[0].datetime <= w[1].datetime) {
            log::warn!("called append_new_logs with unsorted logs: {:#?}", logs);
            return Err(anyhow::anyhow!("logs must be sorted from old to new"));
        }

        // fetch the most recent log in the database to compare against
        let Some(newest_db_log) = self.select_latest_log().await? else {
            // the database is empty, all logs must be new
            self.append_logs(logs).await?;
            return Ok(logs);
        };

        // check if _all_ new logs are actually old
        //
        // if the newest log in the argument is older than the latest
        // log in the database, all logs in the argument must be old.
        if logs.last().map_or(false, |log| {
            log.latest_timestamp_utc() < newest_db_log.latest_timestamp_utc()
        }) {
            return Ok(&[]);
        }

        // check if _all_ new logs are new
        //
        // if the oldest log in the argument is newer than the latest
        // log in the database, all logs in the argument must be new.
        if logs.first().map_or(false, |log| {
            log.earliest_timestamp_utc() > newest_db_log.latest_timestamp_utc()
        }) {
            self.append_logs(logs).await?;
            return Ok(logs);
        }

        // this index is at most `logs.len() - 1` (obviously)
        let most_recent_index = logs
            .iter()
            .position(|log| {
                log.earliest_timestamp_utc() == newest_db_log.earliest_timestamp_utc()
                    && log.message_id == newest_db_log.message_id
                    && log.category_id == newest_db_log.category_id
            })
            .context("couldn't find most recent db log in logs argument")?;

        let candidates = logs.split_at(most_recent_index).1;
        let first_candidate = candidates.first().expect("at least one candidate");
        let update_most_recent = first_candidate.repetition != newest_db_log.repetition;

        // if the repetition changed, update it in the database
        if update_most_recent {
            self.replace_log(&newest_db_log, first_candidate)
                .await
                .context("update most recent db log")?;
        }

        // add all new logs to the database
        self.append_logs(&candidates[1..])
            .await
            .context("insert new logs")?;

        // if we updated the most recent log, include it in the list
        Ok(if update_most_recent {
            candidates
        } else {
            &candidates[1..]
        })
    }

    pub async fn insert_request(&self, req: &Request) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
        INSERT INTO "requests"
        (
            "datetime",
            "name",
            "url",
            "method",
            "duration_ms",
            "response_code",
            "session_id"
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            /* 1 */ req.datetime,
            /* 2 */ req.name,
            /* 3 */ req.url,
            /* 4 */ req.method,
            /* 5 */ req.duration_ms,
            /* 6 */ req.response_code,
            /* 7 */ req.session_id,
        )
        .execute(&self.pool)
        .await
        .context("insert request")?;

        Ok(())
    }

    pub async fn insert_update(&self, update: &Update) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
        INSERT INTO "updates"
        (
            "datetime",
            "upserted_rows"
        )
        VALUES (?1, ?2)
            "#,
            /* 1 */ update.datetime,
            /* 2 */ update.upserted_rows,
        )
        .execute(&self.pool)
        .await
        .context("insert update")?;

        Ok(())
    }
}
