# Enterprise Web3 Service
```
cat  redis-versioned-kv/lua/versioned-kv.lua | redis-cli -h 127.0.0.1 -p 6379 -x FUNCTION LOAD REPLACE
```
## Design

- Redis as store backend
    - Versioned KV on redis
- Scalalbe Web3 service
    - Embedded EVM

### Key Design

#### Account Basic

- address:
    - balance: U256
    - code: U256
    - nonce: U256

Keys:

- `balance:addr.<0x>`
- `code:addr.<0x>`
- `nonce:addr.<0x>`

#### State

- address: H160
    - index: U256
        - value: H256

Keys:

- `state:addr.<0x>:index.<0x>`

#### Transaction

- txhash
    - txobject

Keys:

- `tx:hash.<0x>`

#### Receipt

- txhash
    - receipt object

Keys:

- `receipt:hash.<0x>`

