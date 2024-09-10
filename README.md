# Chonkit

Chunk documents.

## Contents

- [Building](#building)
  - [Local quickstart](#local-quickstart)
- [Running](#running)

## Building

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
```

Creates the 'chunk' directory for storing chunks for inspection.
Exports the `DATABASE_URL` and `VEC_DATABASE_URL` environment variables,
prompting you to pick one vector database provider.

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
