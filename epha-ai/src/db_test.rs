use anyhow::Result;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder};
use qdrant_client::qdrant::{PointStruct, SearchPointsBuilder, UpsertPointsBuilder};
use sea_orm::{ConnectionTrait, Database, Statement};
use std::env;

/// Test Qdrant connection and basic operations
async fn test_qdrant() -> Result<()> {
    println!("Testing Qdrant connection...");

    let qdrant_url = env::var("EPHA_MEMORY_QDRANT_URL").unwrap();
    let client = Qdrant::from_url(&qdrant_url).build()?;

    // Test connection
    let health = client.health_check().await?;
    println!("Qdrant health check: {health:?}");

    // Create test collection
    let collection_name = "test_ephemera";

    if client.collection_exists(collection_name).await? {
        client.delete_collection(collection_name).await?;
        println!("Deleted existing test collection");
    }

    client
        .create_collection(
            CreateCollectionBuilder::new(collection_name)
                .vectors_config(VectorParamsBuilder::new(384, Distance::Cosine)),
        )
        .await?;

    println!("Created test collection: {collection_name}");

    // Test inserting a point
    let points = vec![PointStruct::new(
        1,                                   // Unique point ID
        vec![0.1; 384],                      // Vector
        [("content", "test memory".into())], // Payload
    )];

    client
        .upsert_points(UpsertPointsBuilder::new(collection_name, points))
        .await?;

    println!("Successfully inserted test point");

    // Test search
    let search_request = SearchPointsBuilder::new(
        collection_name, // Collection name
        vec![0.1; 384],  // Search vector
        1,               // Search limit
    )
    .with_payload(true);

    let search_result = client.search_points(search_request).await?;

    println!("Search result count: {}", search_result.result.len());

    // Clean up
    client.delete_collection(collection_name).await?;
    println!("Cleaned up test collection");

    Ok(())
}

/// Test MySQL connection
async fn test_mysql() -> Result<()> {
    println!("Testing MySQL connection...");

    let mysql_url = env::var("EPHA_MEMORY_MYSQL_URL").unwrap();
    let db = Database::connect(&mysql_url).await?;

    // Test simple query
    let result = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            "SELECT 1".to_string(),
        ))
        .await?;

    if !result.is_empty() {
        println!(
            "MySQL connection successful, test query returned: {result:?}"
        );
    } else {
        println!("MySQL connection test failed");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    println!("Starting database connection tests...\n");

    // Test Qdrant
    match test_qdrant().await {
        Ok(_) => println!("\n✅ Qdrant test passed\n"),
        Err(e) => println!("\n❌ Qdrant test failed: {e}\n"),
    }

    // Test MySQL
    match test_mysql().await {
        Ok(_) => println!("✅ MySQL test passed\n"),
        Err(e) => println!("❌ MySQL test failed: {e}\n"),
    }

    println!("Database connection tests completed");
    Ok(())
}
