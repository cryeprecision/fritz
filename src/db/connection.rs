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
        ORDER BY "datetime" DESC
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
    /// Returns a slice over the inserted elements.
    pub async fn append_new_logs<'a>(
        &self,
        logs: &'a [fritz::Log],
    ) -> anyhow::Result<&'a [fritz::Log]> {
        // Database: [3,2,1]
        //
        // [4,5]   -> [5,4,3,2,1]: All logs are new
        // [1,2]   ->     [3,2,1]: All logs are old
        // [2,3,4] ->   [4,3,2,1]: Some logs are new

        let (Some(oldest_api_log), Some(newest_api_log)) = (logs.first(), logs.last()) else {
            log::warn!("called append_new_logs with an empty argument");
            return Ok(&[]);
        };

        // make sure the logs are sorted from old to new
        if !logs.windows(2).all(|w| w[0].datetime <= w[1].datetime) {
            log::warn!("called append_new_logs with unsorted logs: {:#?}", logs);
            return Err(anyhow::anyhow!("logs must be sorted from old to new"));
        }

        let Some(newest_db_log) = self.select_latest_log().await? else {
            // the database is empty, all logs must be new
            self.append_logs(logs).await?;
            return Ok(logs);
        };

        if newest_db_log.datetime < oldest_api_log.datetime {
            // all given logs are new
            self.append_logs(logs).await?;
            return Ok(logs);
        }

        if newest_api_log.datetime < newest_db_log.datetime {
            // all given logs are old
            return Ok(&[]);
        }

        // find the most recent log from the database in the list of
        // new logs to be appended.
        let most_recent_index = logs
            .iter()
            .position(|log| {
                log.datetime == newest_db_log.datetime
                    && log.message_id == newest_db_log.message_id
                    && log.category_id == newest_db_log.category_id
            })
            .context("couldn't find most recent db log in logs argument")?;
        let most_recent = &logs[most_recent_index];
        let update_most_recent = most_recent.repetition != newest_db_log.repetition;

        // if the repetition changed, update it in the database
        if update_most_recent {
            self.replace_log(&newest_db_log, most_recent)
                .await
                .context("update most recent db log")?;
        }

        // insert all new logs into the database
        if most_recent_index != logs.len() - 1 {
            self.append_logs(&logs[most_recent_index + 1..])
                .await
                .context("insert new logs")?;
        }

        Ok(if update_most_recent {
            // we updated the most recent log, so include it in the list
            &logs[most_recent_index..]
        } else {
            &logs[most_recent_index + 1..]
        })
    }
}
