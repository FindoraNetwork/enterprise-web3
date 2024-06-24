CREATE TABLE IF NOT EXISTS common (
    id BIGSERIAL PRIMARY KEY,
    latest_height_key BIGSERIAL NOT NULL,
    lowest_height_key BIGSERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS common_latest_height_key_idx ON common(latest_height_key);
CREATE INDEX IF NOT EXISTS common_lowest_height_key_idx ON common(lowest_height_key);
