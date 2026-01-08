#[cfg(test)]
mod tests {
    use crate::database::entity::post;
    use crate::database::postgres_repo::PostgresPostRepository;
    use apex_core::domain::Post;
    use apex_core::ports::BaseRepository;
    use sea_orm::{DatabaseBackend, MockDatabase};

    #[tokio::test]
    async fn test_find_post_by_id() {
        // Create mock database with expected query results
        let post_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        // Mock the query expectation
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![post::Model {
                id: post_id,
                user_id,
                title: "Test Post".to_owned(),
                content: "Content".to_owned(),
                created_at: now.into(),
                updated_at: now.into(),
            }]])
            .into_connection();

        let repo = PostgresPostRepository::new(db);

        let result: Option<Post> = repo.find_by_id(post_id).await.unwrap();

        assert!(result.is_some());
        let post = result.unwrap();
        assert_eq!(post.title, "Test Post");
        assert_eq!(post.id, post_id);
    }
}
