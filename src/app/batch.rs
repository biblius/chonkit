use super::state::AppState;
use crate::{
    core::service::{
        document::DocumentService,
        vector::{dto::CreateEmbeddings, VectorService},
    },
    error::ChonkitError,
};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use tokio::{select, sync::mpsc};
use uuid::Uuid;

pub type BatchEmbedderHandle = mpsc::Sender<BatchJob>;

pub struct BatchEmbedder {
    /// Job queue.
    q: HashMap<Uuid, mpsc::Sender<JobResult>>,

    /// Job receiver.
    job_rx: mpsc::Receiver<BatchJob>,

    /// Report receiver. Receives an embedding report every time
    /// a document is embedded.
    result_rx: mpsc::Receiver<BatchJobResult>,

    /// Use as the sending end of the report channel.
    /// Here solely so we can just clone it for jobs.
    result_tx: mpsc::Sender<BatchJobResult>,

    state: AppState,
}

impl BatchEmbedder {
    pub fn new(job_rx: mpsc::Receiver<BatchJob>, state: AppState) -> Self {
        let (result_tx, result_rx) = mpsc::channel(128);
        Self {
            q: HashMap::new(),
            job_rx,
            state,
            result_tx,
            result_rx,
        }
    }

    pub fn start(mut self) {
        tokio::spawn(async move {
            loop {
                select! {
                    job = self.job_rx.recv() => {
                        let Some(job) = job else {
                            tracing::info!("Job receiver channel closed, shutting down executor");
                            break;
                        };

                        let job_id = Uuid::new_v4();
                        let state = self.state.clone();
                        let result_tx = self.result_tx.clone();

                        let BatchJob { collection, add, remove, finished_tx } = job;

                        self.q.insert(job_id, finished_tx);

                        tracing::info!("Starting job '{job_id}' | Adding {} | Removing {}", add.len(), remove.len());

                        tokio::spawn(
                            Self::execute_job(job_id, state, add,remove, collection, result_tx)
                        );
                    }

                    result = self.result_rx.recv() => {
                        let Some(result) = result else {
                            tracing::warn!("Job result channel closed, shutting down executor");
                            break;
                        };

                        let result = match result {
                            BatchJobResult::Event(result) => result,
                            BatchJobResult::Done(id) => {
                                self.q.remove(&id);
                                tracing::debug!("Job '{id}' finished, removing from queue");
                                continue;
                            }
                        };

                        let JobEvent { job_id, result } = result;

                        let Some(finished_tx) = self.q.get(&job_id) else {
                            continue;
                        };

                        let result = finished_tx.send(result).await;

                        tracing::debug!("Sent result to channel ({result:?})");
                    }
                }
            }
        });
    }

    async fn execute_job(
        job_id: Uuid,
        state: AppState,
        add: Vec<Uuid>,
        remove: Vec<Uuid>,
        collection_id: Uuid,
        result_tx: mpsc::Sender<BatchJobResult>,
    ) {
        /// Matches the result and continues on error, sending the error to the result channel.
        macro_rules! ok_or_continue {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::debug!("Sending error to channel ({e:?})");
                        let result = JobEvent {
                            job_id,
                            result: JobResult::Err(e),
                        };
                        let _ = result_tx.send(BatchJobResult::Event(result)).await;
                        continue;
                    }
                }
            };
        }

        /// Matches the result and returns on error, sending the error to the result channel.
        macro_rules! ok_or_return {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::debug!("Sending error to channel ({e:?})");
                        let result = JobEvent {
                            job_id,
                            result: JobResult::Err(e),
                        };
                        let _ = result_tx.send(BatchJobResult::Event(result)).await;
                        let _ = result_tx.send(BatchJobResult::Done(job_id)).await;
                        return;
                    }
                }
            };
        }

        let vector_service = VectorService::new(state.postgres.clone());
        let document_service = DocumentService::new(state.postgres.clone());

        let collection = ok_or_return!(vector_service.get_collection(collection_id).await);
        let vector_db = state.vector_db(ok_or_return!(collection.provider.as_str().try_into()));
        let embedder = state.embedder(ok_or_return!(collection.embedder.as_str().try_into()));

        for document_id in add.into_iter() {
            tracing::debug!("Processing document '{document_id}'");

            // Map the existence of the embeddings as an error
            let embeddings = ok_or_continue!(
                vector_service
                    .get_embeddings(document_id, collection_id)
                    .await
            );

            let exists = if embeddings.is_some() {
                Err(ChonkitError::AlreadyExists(format!(
                    "Embeddings for '{document_id}' in collection '{collection_id}'"
                )))
            } else {
                Ok(())
            };

            ok_or_continue!(exists);

            let document = ok_or_continue!(document_service.get_document(document_id).await);

            // Initialize the report so we get the timestamp before the embedding starts
            let report = EmbeddingAddReportBuilder::new(document.id, collection.id);

            let store = state.store(ok_or_continue!(document.src.as_str().try_into()));

            // Get the content and chunk it

            let content = ok_or_continue!(document_service.get_content(&*store, document_id).await);
            let chunks = ok_or_continue!(
                document_service
                    .get_chunks(document.id, &content, Some(embedder.clone()))
                    .await
            );
            let chunks = match chunks {
                crate::core::chunk::ChunkedDocument::Ref(r) => r,
                crate::core::chunk::ChunkedDocument::Owned(ref o) => {
                    o.iter().map(|s| s.as_str()).collect()
                }
            };

            let create = CreateEmbeddings {
                id: document.id,
                collection: collection.id,
                chunks: &chunks,
            };

            let embeddings = ok_or_continue!(
                vector_service
                    .create_embeddings(&*vector_db, &*embedder, create)
                    .await
            );

            let report = report
                .embeddings_id(embeddings.id)
                .model_used(collection.model.clone())
                .vector_db(vector_db.id().to_string())
                .total_chunks(chunks.len())
                .finished_at(Utc::now())
                .build();

            let result = JobEvent {
                job_id,
                result: JobResult::Ok(JobReport::Addition(report)),
            };

            let _ = result_tx.send(BatchJobResult::Event(result)).await;
        }

        for document_id in remove.into_iter() {
            let report = EmbeddingRemovalReportBuilder::new(document_id, collection.id);

            let _ = ok_or_continue!(
                vector_service
                    .delete_embeddings(collection_id, document_id, &*vector_db)
                    .await
            );

            let report = report.finished_at(Utc::now()).build();

            let result = JobEvent {
                job_id,
                result: JobResult::Ok(JobReport::Removal(report)),
            };

            let _ = result_tx.send(BatchJobResult::Event(result)).await;
        }

        let _ = result_tx.send(BatchJobResult::Done(job_id)).await;
    }
}

/// Used for batch embedding jobs.
#[derive(Debug)]
pub struct BatchJob {
    /// Collection ID, i.e. where to store the embeddings.
    collection: Uuid,

    /// Documents to embed and add to the collection.
    add: Vec<Uuid>,

    /// Documents to remove from the collection.
    remove: Vec<Uuid>,

    /// Sends finished document embeddings back to whatever sent the job.
    finished_tx: mpsc::Sender<JobResult>,
}

impl BatchJob {
    pub fn new(
        collection: Uuid,
        add: Vec<Uuid>,
        remove: Vec<Uuid>,
        finished_tx: mpsc::Sender<JobResult>,
    ) -> Self {
        Self {
            collection,
            add,
            remove,
            finished_tx,
        }
    }
}

/// Used internally to track the status of an embedding job.
/// If a `Done` is received, the job is removed from the executor's queue.
#[derive(Debug)]
enum BatchJobResult {
    /// Represents a finished removal or addition job with respect to the document.
    Event(JobEvent),

    /// Represents a completely finished job.
    Done(Uuid),
}

/// Represents a single document embedding result in a job.
#[derive(Debug)]
struct JobEvent {
    /// ID of the job the embedding happened in.
    job_id: Uuid,

    /// Result of the embedding process.
    result: JobResult,
}

/// Result of embedding a single document.
#[derive(Debug)]
pub enum JobResult {
    Ok(JobReport),
    Err(ChonkitError),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JobReport {
    /// Job type for adding documents to a collection.
    Addition(EmbeddingAddReport),

    /// Job type for removing documents from a collection.
    Removal(EmbeddingRemovalReport),
}

#[derive(Debug, Serialize)]
pub struct EmbeddingAddReport {
    pub document_id: Uuid,
    pub collection_id: Uuid,
    pub embeddings_id: Uuid,
    pub model_used: String,
    pub vector_db: String,
    pub total_chunks: usize,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingRemovalReport {
    pub document_id: Uuid,
    pub collection_id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
struct EmbeddingAddReportBuilder {
    document_id: Uuid,
    collection_id: Uuid,
    embeddings_id: Option<Uuid>,
    model_used: Option<String>,
    vector_db: Option<String>,
    total_chunks: Option<usize>,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl EmbeddingAddReportBuilder {
    fn new(document_id: Uuid, collection_id: Uuid) -> Self {
        Self {
            document_id,
            collection_id,
            started_at: chrono::Utc::now(),
            embeddings_id: None,
            model_used: None,
            vector_db: None,
            total_chunks: None,
            finished_at: None,
        }
    }

    fn embeddings_id(mut self, embeddings_id: Uuid) -> Self {
        self.embeddings_id = Some(embeddings_id);
        self
    }

    fn model_used(mut self, model_used: String) -> Self {
        self.model_used = Some(model_used);
        self
    }

    fn vector_db(mut self, vector_db: String) -> Self {
        self.vector_db = Some(vector_db);
        self
    }

    fn finished_at(mut self, finished_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.finished_at = Some(finished_at);
        self
    }

    fn total_chunks(mut self, total_chunks: usize) -> Self {
        self.total_chunks = Some(total_chunks);
        self
    }

    fn build(self) -> EmbeddingAddReport {
        EmbeddingAddReport {
            document_id: self.document_id,
            collection_id: self.collection_id,
            embeddings_id: self.embeddings_id.unwrap(),
            model_used: self.model_used.unwrap(),
            vector_db: self.vector_db.unwrap(),
            total_chunks: self.total_chunks.unwrap(),
            started_at: self.started_at,
            finished_at: self.finished_at.unwrap(),
        }
    }
}

#[derive(Debug)]
struct EmbeddingRemovalReportBuilder {
    document_id: Uuid,
    collection_id: Uuid,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl EmbeddingRemovalReportBuilder {
    fn new(document_id: Uuid, collection_id: Uuid) -> Self {
        Self {
            document_id,
            collection_id,
            started_at: chrono::Utc::now(),
            finished_at: None,
        }
    }

    fn finished_at(mut self, finished_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.finished_at = Some(finished_at);
        self
    }

    fn build(self) -> EmbeddingRemovalReport {
        EmbeddingRemovalReport {
            document_id: self.document_id,
            collection_id: self.collection_id,
            started_at: self.started_at,
            finished_at: self.finished_at.unwrap(),
        }
    }
}
