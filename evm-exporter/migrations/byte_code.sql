CREATE TABLE IF NOT EXISTS byte_code (
    id BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL,
    address CHARACTER VARYING(64) NOT NULL,
    height SERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS byte_code_address_idx ON byte_code(address);
CREATE INDEX IF NOT EXISTS byte_code_height_idx ON byte_code(height);
