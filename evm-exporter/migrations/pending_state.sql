CREATE TABLE IF NOT EXISTS pending_state (
    id BIGSERIAL PRIMARY KEY,
    value CHARACTER VARYING(128) NOT NULL,
    idx CHARACTER VARYING(128) NOT NULL,
    address CHARACTER VARYING(64) NOT NULL
);

CREATE INDEX IF NOT EXISTS pending_state_address_idx ON pending_state(address);
CREATE INDEX IF NOT EXISTS pending_state_idx_idx ON pending_state(idx);
