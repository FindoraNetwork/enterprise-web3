CREATE TABLE IF NOT EXISTS issuance (
    id BIGSERIAL PRIMARY KEY,
    value CHARACTER VARYING(128) NOT NULL,
    height SERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS issuance_height_idx ON issuance(height);
