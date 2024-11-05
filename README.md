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

## General information

Chonkit is an application for chunking documents
whose chunks can then be used for retrieval augmented generation (RAG).

I suggest you read about [text embeddings](https://stackoverflow.blog/2023/11/09/an-intuitive-introduction-to-text-embeddings/) if you don't know what they are.
It will clarify the following explanation.

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

OpenAPI documentation is available at any chonker instance at `http://your-address/swagger-ui`.

## Binaries

Chonkit offers the following binaries:

- Server (`--bin server`); exposes an HTTP API around `chonkit`'s core functionality.
- CLI (`--bin cli`); exposes a CLI interface around `chonkit`'s core functionality.
- Fastembedder (`--bin fembedder`); A small binary used to initiate fastembed in
  CUDA mode on a remote machine that can be used as an embedding API,
  similarly to OpenAI.

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

### Features

The following is a table of the supported build features.

| Feature     | Configuration      | Description                                                                                                                       |
| ----------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `http`      | Execution mode     | Build for http (server) execution mode.                                                                                           |
| `cli`       | Execution mode     | Build for cli execution mode.                                                                                                     |
| `qdrant`    | VectorDb provider  | Enable qdrant as one of the vector database providers.                                                                            |
| `weaviate`  | VectorDb provider  | Enable weaviate as one of the vector database providers.                                                                          |
| `fembed`    | Embedder provider  | Enable fastembed as one of the embedding providers. One of either `fe-local` or `fe-remote` is necessary when enabling this flag. |
| `fe-local`  | Embedder provider  | Use the local implementation of `Embedder` for `FastEmbedder`. Mutually exclusive with `fe-remote`.                               |
| `fe-remote` | Embedder provider  | Use the remote implementation of `Embedder` for `FastEmbedder`. Mutually exclusive with `fe-local`.                               |
| `openai`    | Embedder provider  | Enable openai as one of the embedding providers.                                                                                  |
| `cuda`      | Execution provider | Available when using `fembed`. When enabled, uses the CUDAExecutionProvider for the onnxruntime.                                  |

Full build command example

```bash
cargo build -F http -F qdrant --release
```

Chonkit can be built for 2 execution modes; `cli` and `http` (http is default).
These are selected via feature flags when invoking `cargo` (via the `-F` flag).

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

See the [dockerfile](Dockerfile) and [docker-compose file](docker-compose.yml)
for more details.

### Local quickstart

```bash
source setup.sh
cargo run --bin server
```

Creates the 'chunk' directory for storing chunks for inspection.
Creates the 'upload' directory for storing uploaded documents.
Starts the infrastructure containers (postgres, qdrant, weaviate).
Exports the necessary environment variables to run chonkit.
Starts the http API with the default features; Qdrant and local fastembed.

## Running

Chonkit accepts the following arguments:

| Arg                 | Flag | Description                                           | Env               | Feature     | Default         |
| ------------------- | ---- | ----------------------------------------------------- | ----------------- | ----------- | --------------- |
| `--db-url`          | `-d` | The database URL.                                     | `DATABASE_URL`    | \*          | -               |
| `--log`             | `-l` | The `RUST_LOG` env filter string to use.              | `RUST_LOG`        | \*          | `info`          |
| `--upload-path`     | `-u` | If using the `FsDocumentStore`, sets its upload path. | `UPLOAD_PATH`     | \*          | `./upload`      |
| `--address`         | `-a` | The address (host:port) to bind the server to.        | `ADDRESS`         | `http`      | `0.0.0.0:42069` |
| `--allowed-origins` | `-c` | Comma separated list of origins allowed to connect.   | `ALLOWED_ORIGINS` | `http`      | -               |
| `--qdrant-url`      | `-q` | Qdrant vector database URL.                           | `QDRANT_URL`      | `qdrant`    | -               |
| `--weaviate-url`    | `-w` | Weaviate vector database URL.                         | `WEAVIATE_URL`    | `weaviate`  | -               |
| `--fembed-url`      | `-f` | Remote fastembed URL.                                 | `FEMBED_URL`      | `fe-remote` | -               |
| -                   | -    | OpenAI API key.                                       | `OPENAI_KEY`      | `openai`    | -               |

The arguments have priority over the environment variables.
See `RUST_LOG` syntax [here](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html#configure-logging).
