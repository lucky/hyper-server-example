#[derive(Debug)]
pub enum Errors {
    Database(tokio_postgres::Error),
    Json(serde_json::Error),
    Hyper(hyper::Error),
    Pool(deadpool_postgres::PoolError),
}
