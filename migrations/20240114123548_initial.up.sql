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
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- File name.
    name TEXT NOT NULL,

    -- Absolute path to the file depending on type of file storage.
    path TEXT NOT NULL,

    -- The extension of the document used for parsing.
    ext TEXT NOT NULL,

    -- The content hash of the document.
    hash TEXT NOT NULL,

    -- Document source, e.g. local, minio, etc.
    src TEXT NOT NULL,

    -- A label for grouping together files with the same label.
    -- Documents can have only a single label.
    label TEXT,

    -- Tags for documents for grouping.
    -- Multiple tags are allowed.
    tags TEXT[],

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_src_path_hash UNIQUE (src, path, hash)
);

-- Stores chunking configurations for documents.
CREATE TABLE chunkers(
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    document_id UUID UNIQUE NOT NULL REFERENCES documents ON DELETE CASCADE,

    config JSONB NOT NULL, -- Only god can judge us.

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stores parsing configurations for documents.
CREATE TABLE parsers(
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    document_id UUID UNIQUE NOT NULL REFERENCES documents ON DELETE CASCADE,

    config JSONB NOT NULL, -- Only god can judge us.

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stores vector collection information. 
CREATE TABLE collections(
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- The name of the collection. Unique in combination with provider.
    name TEXT NOT NULL,

    -- The model used to generate the vectors.
    model TEXT NOT NULL,

    -- The embedder whose model is used.
    embedder TEXT NOT NULL,

    -- The vector DB used to store the vectors.
    provider TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_name_provider UNIQUE (name, provider)
);

-- Stores document embedding information. 
CREATE TABLE embeddings(
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    document_id UUID NOT NULL REFERENCES documents ON DELETE CASCADE,

    collection_id UUID NOT NULL REFERENCES collections ON DELETE CASCADE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


SELECT manage_updated_at('documents');
SELECT manage_updated_at('chunkers');
SELECT manage_updated_at('parsers');
SELECT manage_updated_at('collections');
SELECT manage_updated_at('embeddings');
