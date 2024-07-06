# Chonkit

A work-in-progress data pre-processor for LLM pipelines.
Contains text chunkers, persistence for documents and their chunk embeddings (TODO), semantic retrieval (TODO) and LLM prompt pipelines (TODO).

## Build

The quickest way to get the backend up and running is with Docker compose.

First you'll need to create a `config.json` file in the project root which is necessary
to configure the backend. The file is already configured to read the `content` directory from the repository root, so no further configuration should be required when running locally.

```bash
cp config.example.json config.json
```

Then, from the project root, run the following.

```bash
docker compose up -d
```

After that, `cd` to the `web` directory and create your `.env` file.

```bash
cd web
cp .env.example .env
```

The file should already have the correct parameters to get you up and running locally and no further configuration should be necessary.

To run the web interface, run the following command from the `web` directory.

```bash
npm run dev
```
