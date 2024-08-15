CREATE TABLE IF NOT EXISTS block_info (
    id BIGSERIAL PRIMARY KEY,
    block_hash CHARACTER VARYING(128) NOT NULL,
    block_height CHARACTER VARYING(128) NOT NULL,
    block JSONB NOT NULL,
    receipt JSONB NOT NULL,
    statuses JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS block_info_block_hash_idx ON block_info(block_hash);
CREATE INDEX IF NOT EXISTS block_info_block_height_idx ON block_info(block_height);
