//! A pool implementation for `diesel-async` based on [`diesel::r2d2`]
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::result::Error;
//! use futures_util::FutureExt;
//! use diesel_async::pooled_connection::AsyncDieselConnectionManager;
//! use diesel_async::pooled_connection::r2d2::Pool;
//! use diesel_async::{RunQueryDsl, AsyncConnection};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! #     run_test().await.unwrap();
//! # }
//! #
//! # #[cfg(feature = "postgres")]
//! # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncPgConnection> {
//! #     let db_url = database_url_from_env("PG_DATABASE_URL");
//! let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
//! #     config
//! #  }
//! #
//! # #[cfg(feature = "mysql")]
//! # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncMysqlConnection> {
//! #     let db_url = database_url_from_env("MYSQL_DATABASE_URL");
//! #     let config = AsyncDieselConnectionManager::<diesel_async::AsyncMysqlConnection>::new(db_url);
//! #     config
//! #  }
//! #
//! # async fn run_test() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//! #     use schema::users::dsl::*;
//! #     let config = get_config();
//! let pool = Pool::builder().build(config).await?;
//! let mut conn = pool.get().await?;
//! # conn.begin_test_transaction();
//! # create_tables(&mut conn).await;
//! # #[cfg(feature = "mysql")]
//! # conn.begin_test_transaction();
//! let res = users.load::<(i32, String)>(&mut conn).await?;
//! #     Ok(())
//! # }
//! ```

use super::{AsyncDieselConnectionManager, PoolError, PoolableConnection};
use diesel::query_builder::QueryFragment;
use diesel::r2d2::ManageConnection;

/// Type alias for using [`diesel::r2d2::Pool`] with [`diesel-async`]
pub type Pool<C> = diesel::r2d2::Pool<AsyncDieselConnectionManager<C>>;
/// Type alias for using [`diesel::r2d2::PooledConnection`] with [`diesel-async`]
pub type PooledConnection<'a, C> = diesel::r2d2::PooledConnection<AsyncDieselConnectionManager<C>>;

#[async_trait::async_trait]
impl<C> ManageConnection for AsyncDieselConnectionManager<C>
where
    C: PoolableConnection + 'static,
    diesel::dsl::BareSelect<diesel::dsl::AsExprOf<i32, diesel::sql_types::Integer>>:
        crate::methods::ExecuteDsl<C>,
    diesel::query_builder::SqlQuery: QueryFragment<C::Backend>,
{
    type Connection = C;

    type Error = PoolError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        (self.manager_config.custom_setup)(&self.connection_url)
            .map_err(PoolError::ConnectionError)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.ping(&self.manager_config.recycling_method)
            .map_err(PoolError::QueryError)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        std::thread::panicking() || conn.is_broken()
    }
}
