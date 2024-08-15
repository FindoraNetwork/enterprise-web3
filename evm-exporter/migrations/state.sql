CREATE TABLE IF NOT EXISTS state (
    id BIGSERIAL PRIMARY KEY,
    value CHARACTER VARYING(128) NOT NULL,
    idx CHARACTER VARYING(128) NOT NULL,
    address CHARACTER VARYING(64) NOT NULL,
    height SERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS state_address_idx ON state(address);
CREATE INDEX IF NOT EXISTS state_height_idx ON state(height);
CREATE INDEX IF NOT EXISTS state_idx_idx ON state(idx);
