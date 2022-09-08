use redis::{ConnectionLike, FromRedisValue, RedisResult, ToRedisArgs};

pub trait VersionedKVCommand: ConnectionLike + Sized {
    fn vkv_set<K, V>(&mut self, key: K, height: u32, value: V) -> RedisResult<()>
    where
        K: ToRedisArgs,
        V: ToRedisArgs,
    {
        redis::cmd("FCALL")
            .arg("vkv_set")
            .arg(1)
            .arg(key)
            .arg(height)
            .arg(value)
            .query(self)?;

        Ok(())
    }

    fn vkv_get<K, RV>(&mut self, key: K, height: u32) -> RedisResult<RV>
    where
        K: ToRedisArgs,
        RV: FromRedisValue,
    {
        redis::cmd("FCALL")
            .arg("vkv_get")
            .arg(1)
            .arg(key)
            .arg(height)
            .query(self)
    }

    fn vkv_latest<K>(&mut self, key: K) -> RedisResult<u32>
    where
        K: ToRedisArgs,
    {
        let res: u32 = redis::cmd("FCALL")
            .arg("vkv_latest")
            .arg(1)
            .arg(key)
            .query(self)?;
        Ok(res)
    }
}

impl<T: ConnectionLike + Sized> VersionedKVCommand for T {}

pub trait AsyncVersionedKVCommand: ConnectionLike {}

#[cfg(test)]
mod tests {
    use redis::Client;

    use crate::VersionedKVCommand;

    #[test]
    fn test_get_set() {
        let cli = Client::open("redis://127.0.0.1/").unwrap();
        let mut con = cli.get_connection().unwrap();

        let key = "0x12345";

        let v1 = "0xabcd";
        let v2 = "0xefgh";

        con.vkv_set(key, 4, v1).unwrap();
        con.vkv_set(key, 9, v2).unwrap();

        for i in 0..4 {
            let r: Option<String> = con.vkv_get(key, i).unwrap();

            assert_eq!(r, None);
        }
        for i in 4..9 {
            let r: Option<String> = con.vkv_get(key, i).unwrap();

            assert_eq!(r, Some(String::from(v1)));
        }
        for i in 9..15 {
            let r: Option<String> = con.vkv_get(key, i).unwrap();

            assert_eq!(r, Some(String::from(v2)));
        }
    }
}
