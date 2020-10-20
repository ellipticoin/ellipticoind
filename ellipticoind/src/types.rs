pub mod redis {
    pub type Connection = r2d2_redis::r2d2::PooledConnection<r2d2_redis::RedisConnectionManager>;
    pub type Pool = r2d2_redis::r2d2::Pool<r2d2_redis::RedisConnectionManager>;
}
