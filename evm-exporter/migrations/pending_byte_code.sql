CREATE TABLE IF NOT EXISTS pending_byte_code (
    id BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL,
    address CHARACTER VARYING(64) NOT NULL
);

CREATE INDEX IF NOT EXISTS pending_byte_code_address_idx ON pending_byte_code(address);
