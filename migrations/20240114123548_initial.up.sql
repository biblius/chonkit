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

CREATE TABLE documents (
    id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),

    -- File name with extension.
    name TEXT NOT NULL,

    -- Absolute path to the file depending on type of file storage.
    path TEXT NOT NULL,

    -- The extension of the document used for parsing.
    ext TEXT NOT NULL,

    -- A label for grouping together files with the same label.
    -- Documents can have only a single label.
    label TEXT,

    -- Tags for documents for grouping.
    -- Multiple tags are allowed.
    tags TEXT[],

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stores chunking configurations for documents.
CREATE TABLE chunkers(
    id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES documents,
    config JSONB NOT NULL, -- Only god can judge us.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stores parsing configurations for documents.
CREATE TABLE parsers(
    id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES documents,
    config JSONB NOT NULL, -- Only god can judge us.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stores vector collections. 
CREATE TABLE collections(
    id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT manage_updated_at('documents');
SELECT manage_updated_at('chunkers');
SELECT manage_updated_at('parsers');
SELECT manage_updated_at('collections');
