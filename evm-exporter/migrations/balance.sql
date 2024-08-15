CREATE TABLE IF NOT EXISTS balance (
    id BIGSERIAL PRIMARY KEY,
    balance CHARACTER VARYING(128) NOT NULL,
    address CHARACTER VARYING(64) NOT NULL,
    height SERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS balance_address_idx ON balance(address);
CREATE INDEX IF NOT EXISTS balance_height_idx ON balance(height);
