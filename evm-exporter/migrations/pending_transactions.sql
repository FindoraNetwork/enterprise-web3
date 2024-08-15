CREATE TABLE IF NOT EXISTS pending_transactions (
    id BIGSERIAL PRIMARY KEY,
    sign_address CHARACTER VARYING(64) NOT NULL,
    pending_balance CHARACTER VARYING(128) NOT NULL,
    pending_nonce CHARACTER VARYING(128) NOT NULL
);

CREATE INDEX IF NOT EXISTS pending_transactions_sign_address_idx ON pending_transactions(sign_address);
