# Chonkit

Chunk documents.

## Contents

- [General information](#general-information)
- [OpenAPI documentation](#openapi-documentation)
- [Building](#building)
  - [Prerequisites](#prerequisites)
    - [Pdfium](#pdfium)
    - [Fastembed](#fastembed)
    - [CUDA](#cuda)
  - [Features](#features)
  - [Sqlx 'offline' compilation](#sqlx-offline-compilation)
  - [Local quickstart](#local-quickstart)
- [Running](#running)
- [Authorization](#authorization)

## General information

Chonkit is an application for chunking documents
whose chunks can then be used for retrieval augmented generation (RAG).

RAG is a technique to provide LLMs contextual information about arbitrary data.
The jist of RAG is the following:

1. User sends a prompt.
2. Prompt is used for semantic search to retrieve context from the knowledge base.
3. Context and prompt are sent to LLM, providing it the necessary information to
   answer the prompt accurately.

Chonkit focuses on problem 2.

### Parsers

Documents come in many different shapes and sizes. A parser is responsible
for turning its content into bytes (raw text) and forwarding them to the chunkers.
Parsers can be configured to read only a specific range from the document,
and they can be configured to skip arbitrary text elements.

Chonkit provides an API to configure parsers for fast iteration.

### Chunkers

Embedding and retrieving whole documents is unfeasible
as they can be massive, so we need some way to split them up into
smaller parts, but still retain information clarity.

Chonkit currently offers 3 flavors of chunkers:

- SlidingWindow - the simplest (and worst performing) chunking implementation.
- SnappingWindow - a better heuristic chunker that retains sentence stops.
- SemanticWindow - an experimental chunker that uses embeddings and their
  distances to determine chunk boundaries.

The optimal flavor depends on the document being chunked.
There is no perfect chunking flavor and finding the best one will be a game of
trial and error, which is why it is important to get fast feedback when chunking.

Chonkit provides APIs to configure how documents get chunked, as well as a preview
API for fast iteration.

### Vectors

Once the documents are chunked, we have to store them somehow. We do this by
embedding them into vectors and storing them to a collection in a
vector database. Vector databases are specialised software used
for efficient storage of these vectors and their retrieval.

Chonkit provides APIs to manipulate vector collections and store embeddings
into them.

## OpenAPI documentation

OpenAPI documentation is available at any chonkit instance at `http://your-address/swagger-ui`.

## Binaries

This workspace consists the following binaries:

- chonkit; exposes an HTTP API around `chonkit`'s core functionality.
- feserver; used to initiate fastembed with
  CUDA and expose an HTTP API for embeddings.

## Building

### Prerequisites

#### Pdfium

Chonkit depends on [pdfium_render](https://github.com/ajrcarey/pdfium-render)
to parse PDFs. This library depends on [libpdfium.so](https://github.com/bblanchon/pdfium-binaries).
In order for compilation to succeed, the library must be installed on the system.
To download a version of `libpdfium` compatible with chonkit (6666),
run the following (assuming Linux):

```bash
mkdir pdfium
wget https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F6666/pdfium-linux-x64.tgz -O - | tar -xzvf - -C ./pdfium
```

The library can be found in `./pdfium/lib/libpdfium.so`.
In order to let cargo know of its existence, you have 2 options:

- Set the `LD_LIBRARY_PATH` environment variable.

  - By default, the GNU linker is set up to search for libraries in `/usr/lib`.
    If you copy the `libpdfium.so` into one of those directories, you do not
    need to need to set this variable. However, if you want to use the library
    from a different location, you need to tell the linker where it is:

    ```bash
    export LD_LIBRARY_PATH=/path/to/dir/containing/pdfium:$LD_LIBRARY_PATH
    ```

    Note: You need to pass the directory that contains the `libpdfium.so` file,
    not the file itself. This command could also be placed in your `.rc` file.

- Copy the `libpdfium.so` file to `/usr/lib`.

The latter is the preferred option as it is the least involved.

See also: [rpath](https://en.wikipedia.org/wiki/Rpath).

Note: The same procedure is applicable on Mac, only the paths and
actual library files will be different.

#### Fastembed

- Required when compiling with `fe-local`.

Fastembed models require an [onnxruntime](https://github.com/microsoft/onnxruntime).
This library can be downloaded from [here](https://github.com/microsoft/onnxruntime/releases),
or via the system's native package manager.

#### CUDA

- Required when compiling with `fe-local` and `cuda`.

If using the `cuda` feature flag with `fastembed`, the system will need to have
the [CUDA toolkit](https://developer.nvidia.com/cuda-downloads) installed.
Fastembed, and in turn `ort`, will then use the CUDA execution provider for the
onnxruntime. `ort` is designed to fail gracefully if it cannot register CUDA as
one of the execution providers and the CPU provider will be used as fallback.

Additionally, if running `feserver` with Docker, [these instructions](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html#installation)
need to be followed to enable GPUs in Docker.

### Features

The following is a table of the supported build features.

| Feature     | Configuration      | Description                                                                                         |
| ----------- | ------------------ | --------------------------------------------------------------------------------------------------- |
| `qdrant`    | VectorDb provider  | Enable qdrant as one of the vector database providers.                                              |
| `weaviate`  | VectorDb provider  | Enable weaviate as one of the vector database providers.                                            |
| `fe-local`  | Embedder provider  | Use the implementation of `Embedder` with `LocalFastEmbedder`. Mutually exclusive with `fe-remote`. |
| `fe-remote` | Embedder provider  | Use the implementation of `Embedder` with `RemoteFastEmbedder`. Mutually exclusive with `fe-local`. |
| `openai`    | Embedder provider  | Enable openai as one of the embedding providers.                                                    |
| `cuda`      | Execution provider | Available when using `fe-local`. When enabled, uses the CUDAExecutionProvider for the onnxruntime.  |

#### Full build command example

```bash
cargo build -F "qdrant weaviate fe-local" --release
```

### Sqlx 'offline' compilation

By default, Chonkit uses [sqlx](https://github.com/launchbadge/sqlx) with Postgres.
During compilation, sqlx will use the `DATABASE_URL` environment variable to
connect to the database. In order to prevent this default behaviour, run

```bash
cargo sqlx prepare
```

This will cache the queries needed for 'offline' compilation.
The cached queries are stored in the `.sqlx` directory and are checked
into version control. You can check whether the build works by unsetting
the `DATABASE_URL` environment variable.

```bash
unset DATABASE_URL
```

### Local quickstart

```bash
source setup.sh
cargo run --bin server
```

Creates the 'upload' directory for storing uploaded documents.
Starts the infrastructure containers (postgres, qdrant, weaviate).
Exports the necessary environment variables to run chonkit.
Starts the http API with the default features; Qdrant and local fastembed.

## Running

Chonkit accepts the following arguments:

| Arg                 | Flag | Env               | Feature     | Default         | Description                                           |
| ------------------- | ---- | ----------------- | ----------- | --------------- | ----------------------------------------------------- |
| `--db-url`          | `-d` | `DATABASE_URL`    | \*          | -               | The database URL.                                     |
| `--log`             | `-l` | `RUST_LOG`        | \*          | `info`          | The `RUST_LOG` env filter string to use.              |
| `--upload-path`     | `-u` | `UPLOAD_PATH`     | \*          | `./upload`      | If using the `FsDocumentStore`, sets its upload path. |
| `--address`         | `-a` | `ADDRESS`         | \*          | `0.0.0.0:42069` | The address (host:port) to bind the server to.        |
| `--allowed-origins` | `-c` | `ALLOWED_ORIGINS` | \*          | -               | Comma separated list of origins allowed to connect.   |
| `--qdrant-url`      | `-q` | `QDRANT_URL`      | `qdrant`    | -               | Qdrant vector database URL.                           |
| `--weaviate-url`    | `-w` | `WEAVIATE_URL`    | `weaviate`  | -               | Weaviate vector database URL.                         |
| `--fembed-url`      | `-f` | `FEMBED_URL`      | `fe-remote` | -               | Remote fastembed URL.                                 |
| -                   | -    | `OPENAI_KEY`      | `openai`    | -               | OpenAI API key.                                       |

The arguments have priority over the environment variables.
See `RUST_LOG` syntax [here](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html#configure-logging).
See [Authorization](#authorization) for more information about authz specific arguments.

## Authorization

By default, Chonkit does not use any authentication mechanisms. This is
fine for local deployments, but is problematic when chonkit is exposed to
the outside world. The following is a list of supported authorization mechanisms.

### Vault JWT Authorization

**Feature**: `auth-vault`

### Required variables

| Arg                 | Env               | Description                                                               |
| ------------------- | ----------------- | ------------------------------------------------------------------------- |
| `--vault-url`       | `VAULT_URL`       | The endpoint of the vault server.                                         |
| `--vault-role-id`   | `VAULT_ROLE_ID`   | Role ID for the application. Used to log in and obtain an access token.   |
| `--vault-secret-id` | `VAULT_SECRET_ID` | Secret ID for the application. Used to log in and obtain an access token. |
| `--vault-key-name`  | `VAULT_KEY_NAME`  | Name of the key to use for verifying signatures.                          |

### Description

Chonkit can be configured to hook up to Hashicorp's [Vault](https://www.vaultproject.io/)
with [approle](https://developer.hashicorp.com/vault/docs/auth/approle) authentication.
If enabled, at the start of the application Chonkit will log in to the vault and
middleware will be registered on all routes. The middleware will check for the
existence of a token in the following request parameters:

- A cookie with the name `chonkit_access_token` (for web clients).
  If using this, the web frontend must be deployed on the same domain as Chonkit.
- `Authorization` request header (Bearer) (for API clients).

The token is expected to be a valid JWT signed by Vault's
[transit engine](https://developer.hashicorp.com/vault/docs/secrets/transit).
The JWT must contain the version of the key used to sign it, specified by the
`version` claim.

**The signature must have the `vault:vN:` prefix stripped,
Chonkit will add it when verifying using the `version` claim.**

If the signature is valid, additional claims are checked to ensure the
validity of the token ( expiration, audience, etc.).
Specifically, it checks for the following claims:

- `aud == chonkit`
- `exp > now`

To summarize:

1. An authorization server, i.e. an endpoint that generates JWTs intended to be used
   by Chonkit is set up on the same Vault as Chonkit.

2. An application that intends to use Chonkit obtains the access token.

3. The authorization server uses the [sign](https://developer.hashicorp.com/vault/api-docs/secret/transit#sign-data) endpoint
   to generate a signature for a JWT payload and constructs the JWT with it.

4. Chonkit uses the [verify](https://developer.hashicorp.com/vault/api-docs/secret/transit#verify-signed-data) endpoint
   to verify the token signature on the same Vault mount the data was signed.
