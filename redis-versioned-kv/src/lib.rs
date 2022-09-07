use redis::{ConnectionLike, FromRedisValue, RedisResult, ToRedisArgs};

pub trait VersionedKVCommand: ConnectionLike + Sized {
    fn vkv_set<K, V, RV>(&mut self, key: K, height: u32, value: V) -> RedisResult<RV>
    where
        K: ToRedisArgs,
        V: ToRedisArgs,
        RV: FromRedisValue,
    {
        redis::cmd("FCALL")
            .arg("vkv_set")
            .arg(key)
            .arg(height)
            .arg(value)
            .query(self)
    }

    fn vkv_get<K, V, RV>(&mut self, key: K, height: u32) -> RedisResult<RV>
    where
        K: ToRedisArgs,
        V: ToRedisArgs,
        RV: FromRedisValue,
    {
        redis::cmd("FCALL")
            .arg("vkv_get")
            .arg(key)
            .arg(height)
            .query(self)
    }

    fn vkv_latest<K>(&mut self, key: K) -> RedisResult<u32>
    where
        K: ToRedisArgs,
    {
        let res: u32 = redis::cmd("FCALL").arg("vkv_latest").arg(key).query(self)?;
        Ok(res)
    }
}

impl<T: ConnectionLike + Sized> VersionedKVCommand for T {}

pub trait AsyncVersionedKVCommand: ConnectionLike {}
