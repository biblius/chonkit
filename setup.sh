mkdir chunks &> /dev/null

if [[ $? == 0 ]]; then 
	echo "Created directory 'chunks'"
fi

mkdir upload &> /dev/null

if [[ $? == 0 ]]; then 
	echo "Created directory 'upload'"
fi

docker compose -f infra-compose.yml up -d

echo "Select '[w]eaviate' or '[q]drant' (qdrant):" 
read choice

if [[ $choice == 'weaviate' || $choice == 'w' ]]; then
	export VEC_DATABASE_URL=http://localhost:8080
elif [[ $choice == 'qdrant' || $choice == 'q' ]]; then
	export VEC_DATABASE_URL=http://localhost:6334
else
	export VEC_DATABASE_URL=http://localhost:6334
fi

export DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
export RUST_LOG=info,h2=off,lopdf=off,chonkit=debug
