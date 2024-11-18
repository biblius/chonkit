#!/bin/bash

mkdir chunks &> /dev/null

if [[ $? == 0 ]]; then 
	echo "Created directory 'chunks'"
fi

mkdir upload &> /dev/null

if [[ $? == 0 ]]; then 
	echo "Created directory 'upload'"
fi

docker compose up -d

echo "Enter your OpenAI API key (press Enter to skip):"
read -s oai_key

if [[ -z $oai_key ]]; then
	echo "Skipping OPENAI_KEY"
else 
	export OPENAI_KEY=$oai_key
	echo "OPENAI_KEY successfully set"
fi

echo "Enter the remote fembedder URL (press Enter to skip):"
read fembed_url

if [[ -z $fembed_url ]]; then
	echo "Defaulting FEMBED_URL to 127.0.0.1:6969"
	export FEMBED_URL=127.0.0.1:6969
else 
	export FEMBED_URL=$fembed_url
fi


export DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
export QDRANT_URL=http://localhost:6334
export WEAVIATE_URL=http://localhost:8080
export RUST_LOG=info,sqlx=off,h2=off,chonkit=debug
export UPLOAD_PATH=upload
export ADDRESS=0.0.0.0:42069

echo "DATABASE_URL set to $DATABASE_URL"
echo "QDRANT_URL set to $QDRANT_URL"
echo "WEAVIATE_URL set to $WEAVIATE_URL"
echo "VEC_DATABASE_URL set to $VEC_DATABASE_URL"
echo "RUST_LOG set to $RUST_LOG"
echo "UPLOAD_PATH set to '$UPLOAD_PATH'"
echo "ADDRESS set to $ADDRESS"
echo "FEMBED_URL set to $FEMBED_URL"
