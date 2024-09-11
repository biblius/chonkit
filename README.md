# Chonkit

Chunk documents.

## Contents

- [OpenAPI documentation](#openapi-documentation)
- [Building](#building)
  - [Prerequisites](#prerequisites)
    - [Pdfium](#pdfium)
    - [Fastembed](#fastembed)
  - [Features](#features)
  - [Sqlx 'offline' compilation](#sqlx-offline-compilation)
  - [Local quickstart](#local-quickstart)
- [Running](#running)

## OpenAPI documentation

OpenAPI documentation is available at any chonker instance at `http://your-address/swagger-ui`.

## Building

### Prerequisites

#### Pdfium

Chonkit depends on [pdfium_render](https://github.com/ajrcarey/pdfium-render) to parse PDFs.
This library depends on [libpdfium.so](https://github.com/bblanchon/pdfium-binaries).
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

  - By default, the GNU linker is set up to search for libraries in `/usr/lib` and `/usr/local/lib`.
    If you copy the `libpdfium.so` into one of those directories, you do not need to set this variable.
    However, if you want to use the library from a different location, you need to tell the linker where it is:

    ```bash
    export LD_LIBRARY_PATH=/path/to/dir/containing/pdfium:$LD_LIBRARY_PATH
    ```

    Note: You need to pass the directory that contains the `libpdfium.so` file, not the file itself.
    This command could also be placed in your `.rc` file.

- Copy the `libpdfium.so` file to `/usr/lib` or `/usr/local/lib`.

The latter is the preferred option as it is the least involved.

Note: The same procedure is applicable on Mac, only the paths and actual library files will be different.

#### Fastembed

Fastembed requires [onnxruntime](https://github.com/microsoft/onnxruntime).
This library can be downloaded from [here](https://github.com/microsoft/onnxruntime/releases), or via
the system's native package manager.

### Features

The following is a table of the supported build features.

| Feature    | Configuration     | Description                                         |
| ---------- | ----------------- | --------------------------------------------------- |
| `http`     | Execution mode    | Build for http (server) execution mode              |
| `cli`      | Execution mode    | Build for cli execution mode                        |
| `qdrant`   | VectorDb provider | Build with qdrant as the vector database provider   |
| `weaviate` | VectorDb provider | Build with weaviate as the vector database provider |

Full build command example

```bash
cargo build -F http -F qdrant --release
```

Chonkit can be built for 2 execution modes; `cli` and `http` (http is default).
These are selected via feature flags when invoking `cargo` (via the `-F` flag).

### Sqlx 'offline' compilation

By default, Chonkit uses [sqlx](https://github.com/launchbadge/sqlx) with Postgres.
During compilation, sqlx will use the `DATABASE_URL` environment variable to connect to the database.
In order to prevent this default behaviour, run

```bash
cargo sqlx prepare
```

This will cache the queries needed for 'offline' compilation. The cached queries are stored in the `.sqlx`
directory and are checked into version control. You can check whether the build works by unsetting
the `DATABASE_URL` environment variable.

```bash
unset DATABASE_URL
```

See the [dockerfile](Dockerfile) and [docker-compose file](docker-compose.yml) for more details.

### Local quickstart

```bash
source setup.sh
cargo run
```

Creates the 'chunk' directory for storing chunks for inspection.
Starts the infrastructure containers (postgres, qdrant, weaviate).
Exports the necessary environment variables to run chonkit.
Starts the app in `http` mode with `qdrant` as the vector database provider.

## Running

Chonkit accepts the following arguments:

| Arg             | Flag | Description                                           | Env                | Mode   | Default                               |
| --------------- | ---- | ----------------------------------------------------- | ------------------ | ------ | ------------------------------------- |
| `--db-url`      | `-d` | The database URL.                                     | `DATABASE_URL`     | \*     | -                                     |
| `--vec-db-url`  | `-v` | The vector database URL.                              | `VEC_DATABASE_URL` | \*     | -                                     |
| `--log`         | `-l` | The `RUST_LOG` env filter string to use.              | `RUST_LOG`         | \*     | `info,h2=off,lopdf=off,chonkit=debug` |
| `--upload-path` | `-u` | If using the `FsDocumentStore`, sets the upload path. | `UPLOAD_PATH`      | \*     | `./upload`                            |
| `--address`     | `-a` | The address (host:port) to bind the server to.        | `ADDRESS`          | `http` | `0.0.0.0:42069`                       |

The arguments have priority over the environment variables.
See `RUST_LOG` syntax [here](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html#configure-logging).
