use db::DBService;
use uuid::Uuid;

const DEFAULT_STATUSES: &[(&str, &str, i32, bool)] = &[
    ("Backlog", "220 9% 46%", 0, true),
    ("To do", "217 91% 60%", 1, false),
    ("In progress", "38 92% 50%", 2, false),
    ("In review", "258 90% 66%", 3, false),
    ("Done", "142 71% 45%", 4, false),
    ("Cancelled", "0 84% 60%", 5, true),
];

const DEFAULT_TAGS: &[(&str, &str)] = &[
    ("bug", "355 65% 53%"),
    ("feature", "124 82% 30%"),
    ("documentation", "205 100% 40%"),
    ("enhancement", "181 72% 78%"),
];

pub async fn initialize_if_empty(db: &DBService) -> Result<(), sqlx::Error> {
    let pool = &db.pool;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        return Ok(());
    }

    let mut tx = pool.begin().await?;

    let org_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"vibe-kanban-local");
    sqlx::query(
        r#"INSERT INTO organizations (id, name, slug, issue_prefix)
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(org_id)
    .bind("My Workspace")
    .bind("local-workspace")
    .bind("VK")
    .execute(&mut *tx)
    .await?;

    let project_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO projects (id, name, color, issue_counter, organization_id)
           VALUES ($1, $2, $3, 0, $4)"#,
    )
    .bind(project_id)
    .bind("Main Project")
    .bind("217 91% 60%")
    .bind(org_id)
    .execute(&mut *tx)
    .await?;

    for (name, color, sort_order, hidden) in DEFAULT_STATUSES {
        sqlx::query(
            r#"INSERT INTO project_statuses (id, project_id, name, color, sort_order, hidden)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(Uuid::new_v4())
        .bind(project_id)
        .bind(*name)
        .bind(*color)
        .bind(*sort_order)
        .bind(hidden)
        .execute(&mut *tx)
        .await?;
    }

    for (name, color) in DEFAULT_TAGS {
        sqlx::query(
            r#"INSERT INTO kanban_tags (id, project_id, name, color)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(Uuid::new_v4())
        .bind(project_id)
        .bind(*name)
        .bind(*color)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!("First boot: seeded default organization, project, statuses, and tags");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    use super::*;

    async fn setup_pool() -> sqlx::SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .journal_mode(SqliteJournalMode::Memory);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();

        let migrator = sqlx::migrate!("../db/migrations");
        migrator.run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_seeds_on_empty_db() {
        let pool = setup_pool().await;
        let db = DBService { pool };
        initialize_if_empty(&db).await.unwrap();

        let org_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(org_count, 1);

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(project_count, 1);

        let status_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_statuses")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(status_count, 6);

        let tag_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kanban_tags")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(tag_count, 4);

        let prefix: String = sqlx::query_scalar("SELECT issue_prefix FROM organizations")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(prefix, "VK");

        let counter: i32 = sqlx::query_scalar("SELECT issue_counter FROM projects")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(counter, 0);
    }

    #[tokio::test]
    async fn test_no_duplicates_on_second_call() {
        let pool = setup_pool().await;
        let db = DBService { pool };

        initialize_if_empty(&db).await.unwrap();
        initialize_if_empty(&db).await.unwrap();

        let org_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(org_count, 1);

        let status_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_statuses")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(status_count, 6);

        let tag_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kanban_tags")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(tag_count, 4);
    }
}
