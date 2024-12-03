#!/bin/bash

if [[ " $@ " == *" -h "* ]]; then
	echo "Usage: source setup.sh [-h] [-c] [-v]"
	echo "  -h  Show this help message."
	echo "  -e  Source the .env file in the directory of running the script and skip all prompts."
	echo "  -c  Clean up all environment variables set by this script."
	echo "  -v  Vault setup."
	return 0
fi

if [[ " $@ " == *" -c "* ]]; then
    echo "Cleaning up environment variables."
    unset DATABASE_URL
    unset QDRANT_URL
    unset WEAVIATE_URL
    unset OPENAI_KEY
    unset FEMBED_URL
    unset VAULT_URL
    unset VAULT_ROLE_ID
    unset VAULT_SECRET_ID
    echo "Cleaned up environment variables."
    return 0
fi

function read_arg() {
    local message=$1
    local env_var=$2
    local secret=$3
    local default=$4

    echo "$message"
    if [[ $secret = true ]] ; then
	read -s input
    else
	read -r input
    fi

    if [[ -z $input ]]; then
        if [[ -n $default ]]; then
            echo "Defaulting $env_var to $default"
            export "$env_var"="$default"
        else
	    # Hacky way to get the current value
	    eval "local current_value=\$$env_var"

	    if [[ -n $current_value ]]; then
		echo "$env_var already set, skipping."
		return 0
	    fi

            echo "No value provided for $env_var and no default exists, skipping."
        fi
    else
        export "$env_var"="$input"
    fi
}

mkdir upload &> /dev/null

if [[ $? == 0 ]]; then 
    echo "Created directory 'upload'"
fi

docker compose up -d

# Setting other environment variables
export DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres
echo "DATABASE_URL set to $DATABASE_URL"
export QDRANT_URL=http://localhost:6334
echo "QDRANT_URL set to $QDRANT_URL"
export WEAVIATE_URL=http://localhost:8080
echo "WEAVIATE_URL set to $WEAVIATE_URL"
export RUST_LOG=info,sqlx=off,h2=off,chonkit=debug
echo "RUST_LOG set to $RUST_LOG"
export UPLOAD_PATH=upload
echo "UPLOAD_PATH set to '$UPLOAD_PATH'"
export ADDRESS=0.0.0.0:42069
echo "ADDRESS set to $ADDRESS"

if [[ " $@ " == *" -e "* ]]; then
    cat .env &> /dev/null

    if [[ $? == 0 ]]; then 
	    source .env
    else
	    echo "No .env file found."
	    return 1
    fi

    return 0
fi

FEMBED_DEFAULT="http://127.0.0.1:6969"

read_arg "Enter your OpenAI API key (press Enter to skip)" OPENAI_KEY true
read_arg "Enter the remote fembedder URL (press Enter to default to $FEMBED_DEFAULT)" FEMBED_URL false "$FEMBED_DEFAULT"

if [[ " $@ " == *" -v "* ]]; then
    echo "Running Vault setup."
    read_arg "Enter the Vault URL (leave blank to leave as is)" VAULT_URL
    read_arg "Enter your Vault role ID (leave blank to leave as is)" VAULT_ROLE_ID
    read_arg "Enter your Vault secret ID (leave blank to leave as is)" VAULT_SECRET_ID
    read_arg "Enter the key name to use in the Vault Transit Engine (leave blank to leave as is)" VAULT_KEY_NAME
else
    echo "Skipping Vault setup."
fi
