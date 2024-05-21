use std::time::Duration;
use log::info;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub type SeaQuery = sea_orm::sea_query::query::Query;

pub async fn open_db(schema: &str) -> DatabaseConnection {
    let mut opt = ConnectOptions::new(schema);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .sqlx_logging(false);
    let db = Database::connect(opt).await.expect("数据库打开失败");
    info!("Database connected");
    db
}
