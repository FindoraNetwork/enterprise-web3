CREATE TABLE IF NOT EXISTS allowances (
    id BIGSERIAL PRIMARY KEY,
    owner CHARACTER VARYING(64) NOT NULL,
    spender CHARACTER VARYING(64) NOT NULL,
    value CHARACTER VARYING(128) NOT NULL,
    height BIGSERIAL NOT NULL
);

CREATE INDEX IF NOT EXISTS allowances_owner_idx ON allowances(owner);
CREATE INDEX IF NOT EXISTS allowances_spender_idx ON allowances(spender);
CREATE INDEX IF NOT EXISTS allowances_height_idx ON allowances(height);
