//! Database-related functions
use crate::config::{Config, CONFIG};
use axum::{Extension, Router};
use axum_extra::extract::cookie::Key;
use diesel::{
    mysql::MysqlConnection,
    pg::PgConnection,
    r2d2::{ConnectionManager, PoolError},
    sqlite::SqliteConnection,
    Connection,
};

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
#[serde(field_identifier, rename_all = "lowercase")]
pub enum DatabaseConnection {
    Cockroach,
    Mysql,
    Postgres,
    Sqlite,
}

pub type Pool<T> = r2d2::Pool<ConnectionManager<T>>;
pub type CockroachPool = Pool<PgConnection>;
pub type MysqlPool = Pool<MysqlConnection>;
pub type PostgresPool = Pool<PgConnection>;
pub type SqlitePool = Pool<SqliteConnection>;

#[cfg(feature = "cockraoch")]
pub type PoolType = CockroachPool;

#[cfg(feature = "mysql")]
pub type PoolType = MysqlPool;

#[cfg(feature = "postgres")]
pub type PoolType = PostgresPool;

#[cfg(feature = "sqlite")]
pub type PoolType = SqlitePool;

#[derive(Clone)]
pub enum InferPool {
    Cockroach(CockroachPool),
    Mysql(MysqlPool),
    Postgres(PostgresPool),
    Sqlite(SqlitePool),
}

impl InferPool {
    pub fn init_pool(config: Config) -> Result<Self, r2d2::Error> {
        match config.database {
            DatabaseConnection::Cockroach => {
                init_pool::<PgConnection>(config).map(InferPool::Cockroach)
            }
            DatabaseConnection::Mysql => init_pool::<MysqlConnection>(config).map(InferPool::Mysql),
            DatabaseConnection::Postgres => {
                init_pool::<PgConnection>(config).map(InferPool::Postgres)
            }
            DatabaseConnection::Sqlite => {
                init_pool::<SqliteConnection>(config).map(InferPool::Sqlite)
            }
        }
        .map_err(Into::into)
    }
}

pub fn init_pool<T>(config: Config) -> Result<Pool<T>, PoolError>
where
    T: Connection + 'static,
{
    let manager = ConnectionManager::<T>::new(config.database_url);
    Pool::builder().build(manager)
}

/// Add the connection pool to the router so handlers can extract it
pub fn add_pool(router: Router<Key>) -> Router<Key> {
    let pool = InferPool::init_pool(CONFIG.clone()).expect("Failed to create connection pool");
    match pool {
        InferPool::Cockroach(cockroach_pool) => router.layer(Extension(cockroach_pool)),
        InferPool::Mysql(mysql_pool) => router.layer(Extension(mysql_pool)),
        InferPool::Postgres(postgres_pool) => router.layer(Extension(postgres_pool)),
        InferPool::Sqlite(sqlite_pool) => router.layer(Extension(sqlite_pool)),
    }
}
