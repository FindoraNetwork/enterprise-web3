CREATE TABLE IF NOT EXISTS transactions (
    id BIGSERIAL PRIMARY KEY,
    transaction_hash CHARACTER VARYING(128) NOT NULL,
    transaction_index JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS transactions_transaction_hash_idx ON transactions(transaction_hash);
