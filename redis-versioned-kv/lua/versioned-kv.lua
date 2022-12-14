#!lua name=versioned_kv

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
local function vkv_get(keys, args)
    local key = keys[1]
    local height = args[1]

    local res = redis.call('ZRANGE', key, height, '-inf', 'BYSCORE', 'REV', 'LIMIT', 0, 1)
    if #res ~= 0 then
        local value_key = res[1]

        return redis.call('GET', value_key)
    else
        return nil
    end
end

-- vkv_del <key> <height>
local function vkv_del(keys, args)
    local key = keys[1]
    local height = args[1]

    local res = redis.call('ZRANGE', key, height, '-inf', 'BYSCORE', 'REV', 'LIMIT', 0, 1)
    if #res ~= 0 then
        local value_key = res[1]
        local value = redis.call('GET', value_key);

        local val_keys = redis.call('ZRANGE', key, height, '-inf', 'BYSCORE', 'REV')
        for i,val_key in pairs(val_keys) do
            redis.call('ZREM', key, val_key);
            redis.call('DEL', val_key)
        end;

        local heighted_key = string.format("%s:%08X", key, height)
        redis.call('ZADD', key, height, heighted_key)
        redis.call('SET', heighted_key, value)
    end
end

redis.register_function("vkv_set", vkv_set)
redis.register_function("vkv_get", vkv_get)
redis.register_function("vkv_del", vkv_del)

