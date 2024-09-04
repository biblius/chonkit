mkdir chunks
mkdir test_docs
docker compose -f infra-compose.yml up -d
export DATABASE_URL=postgresql://postgres:postgres@localhost:5433/chonkit

