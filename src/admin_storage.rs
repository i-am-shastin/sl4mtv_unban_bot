use std::sync::Arc;

use sqlx::sqlite::{SqlitePool, SqliteQueryResult};
use anyhow::Result;
use teloxide::types::{MessageId, UserId};

pub struct AdminStorage {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct AdminDbRow {
    // request_msg_id: i32,
    // admin_id: i64,
    reply_msg_id: i32,
    reply: String,
}

impl AdminStorage {
    pub async fn open(path: &str) -> Result<Arc<Self>> {
        let pool = SqlitePool::connect(format!("sqlite:{path}?mode=rwc").as_str()).await?;
        sqlx::query(
            "
CREATE TABLE IF NOT EXISTS admin_replies (
    request_msg_id BIGINT NOT NULL,
    admin_id BIGINT NOT NULL,
    reply_msg_id BIGINT NOT NULL,
    reply TEXT NOT NULL,
    PRIMARY KEY(admin_id, request_msg_id)
);
            "
        )
        .execute(&pool)
        .await?;

        Ok(Arc::new(Self { pool }))
    }

    pub async fn save_admin_message(&self, admin_id: UserId, request_msg_id: MessageId, reply_msg_id: MessageId, reply: &str) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query(
            "
        INSERT INTO admin_replies (admin_id, request_msg_id, reply_msg_id, reply) VALUES (?, ?, ?, ?)
        ON CONFLICT(admin_id, request_msg_id) DO UPDATE SET reply=excluded.reply, reply_msg_id=excluded.reply_msg_id
                "
            )
            .bind(admin_id.0 as i64)
            .bind(request_msg_id.0)
            .bind(reply_msg_id.0)
            .bind(reply)
            .execute(&self.pool)
            .await
    }

    pub async fn get_admin_message(&self, admin_id: UserId, request_msg_id: MessageId) -> Result<Option<(MessageId, String)>> {
        let admin_db_row = sqlx::query_as::<_, AdminDbRow>(
            "SELECT reply_msg_id, reply FROM admin_replies WHERE admin_id = ? AND request_msg_id = ?"
            )
            .bind(admin_id.0 as i64)
            .bind(request_msg_id.0)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = admin_db_row {
            self.delete_admin_message(admin_id, request_msg_id).await?;
            Ok(Some((MessageId(row.reply_msg_id), row.reply)))
        } else {
            Ok(None)
        }
    }

    async fn delete_admin_message(&self, admin_id: UserId, request_msg_id: MessageId) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM admin_replies WHERE admin_id = ? AND request_msg_id = ?")
            .bind(admin_id.0 as i64)
            .bind(request_msg_id.0)
            .execute(&self.pool)
            .await
    }
}