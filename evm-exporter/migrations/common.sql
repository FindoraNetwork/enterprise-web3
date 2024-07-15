CREATE TABLE IF NOT EXISTS common (
    latest_height BIGSERIAL NOT NULL, 
    lowest_height BIGSERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS common_latest_height_idx ON common(latest_height);
CREATE INDEX IF NOT EXISTS common_lowest_height_idx ON common(lowest_height);

INSERT INTO common(latest_height, lowest_height) VALUES (0, 0);
