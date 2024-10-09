use model::{RedisKey, RedisValue};
use redis::{AsyncCommands, Client};
use shared::{config::RedisConfig, error::AppResult};

pub mod model;

pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    pub fn new(config: &RedisConfig) -> AppResult<Self> {
        let client = Client::open(format!("redis://{}:{}", config.host, config.port))?;

        Ok(Self { client })
    }

    pub async fn set_with_ex<T: RedisKey>(
        &self,
        key: &T,
        value: &T::Value,
        ttl: u64,
    ) -> AppResult<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        // INFO: `dependency_on_unit_never_type_fallback` を回避するため、() = で型を指定する
        // @see https://github.com/redis-rs/redis-rs/issues/1228
        () = conn.set_ex(key.inner(), value.inner(), ttl).await?;

        Ok(())
    }

    pub async fn get<T: RedisKey>(&self, key: &T) -> AppResult<Option<T::Value>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let result: Option<String> = conn.get(key.inner()).await?;

        result.map(T::Value::try_from).transpose()
    }

    pub async fn delete<T: RedisKey>(&self, key: &T) -> AppResult<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        () = conn.del(key.inner()).await?;

        Ok(())
    }
}
