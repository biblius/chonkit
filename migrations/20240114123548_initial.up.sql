CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION manage_updated_at(_tbl regclass) RETURNS VOID AS $$
BEGIN
  EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s FOR EACH ROW EXECUTE PROCEDURE set_updated_at()', _tbl);
END;
$$ LANGUAGE plpgsql;

CREATE TABLE files (
    id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),

    -- File name with extension, directories do not have names
    name TEXT NOT NULL,

    -- Which directory the file belongs to, NULL if in root directory
    parent UUID REFERENCES files(id) ON DELETE CASCADE ON UPDATE CASCADE, 

    -- Absolute path in the fs
    path TEXT NOT NULL,

    -- Tags for directories apply to all documents in it
    tags TEXT[],

    is_dir BOOLEAN NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT manage_updated_at('files');
