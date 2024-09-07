mkdir chunks &> /dev/null

if [[ $? == 0 ]]; then 
	echo "Created directory 'chunks'"
fi

mkdir upload &> /dev/null

if [[ $? == 0 ]]; then 
	echo "Created directory 'upload'"
fi

docker compose -f infra-compose.yml up -d

export DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
export QDRANT_URL=http://localhost:6334
export RUST_LOG=info,h2=off,lopdf=off,chonkit=debug
