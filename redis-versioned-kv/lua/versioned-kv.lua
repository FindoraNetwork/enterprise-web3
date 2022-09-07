#!lua name=versioned-kv

-- vkv_set <key> <height> <value>
local function vkv_set (keys, args)
    local key = keys[1]
    local height = args[1]

    local heighted_key = string.format("%s:%08X", key, height)

    local value = args[2]

    redis.call('ZADD', key, height, heighted_key)

    redis.call('SET', heighted_key, value)
end

-- vkv_get <key> <height> -> <value>
-- vkv_get <key> latest -> <value>
local function vkv_get(keys, args)
    local key = keys[1]
    local height = args[1]

    if height == 'latest' then
        height = '+inf'
    end

    local res = redis.call('ZRANGE', key, height, '-inf', 'BYSCORE', 'REV', 'LIMIT', 0, 1)
    local value_key = res[1]

    return redis.call('GET', value_key)
end

-- vkv_latest <key> -> <height>
local function vkv_latest(keys)
    local key = keys[1]

    local res = redis.call('ZRANGE', key, '+inf', '-inf', 'BYSCORE', 'REV', 'LIMIT', 0, 1)
    local value_key = res[1]

    return redis.call('ZSCORE', value_key)
end

redis.register_function("vkv_set", vkv_set)
redis.register_function("vkv_get", vkv_get)
redis.register_function("vkv_latest", vkv_latest)

