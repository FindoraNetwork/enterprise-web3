CREATE TABLE IF NOT EXISTS nonce (
    id BIGSERIAL PRIMARY KEY,
    nonce CHARACTER VARYING(128) NOT NULL,
    address CHARACTER VARYING(64) NOT NULL,
    height BIGSERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS nonce_address_idx ON nonce(address);
CREATE INDEX IF NOT EXISTS nonce_height_idx ON nonce(height);
