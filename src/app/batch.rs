use super::service::AppState;
use crate::{
    core::service::{
        document::DocumentService,
        vector::{dto::CreateEmbeddings, VectorService},
    },
    error::ChonkitError,
};
use chrono::Utc;
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::{select, sync::mpsc};
use uuid::Uuid;

pub type BatchEmbedderHandle = mpsc::Sender<EmbeddingJob>;

pub struct BatchEmbedder {
    /// Job queue.
    q: HashMap<Uuid, mpsc::Sender<EmbeddingResult>>,

    /// Job receiver.
    job_rx: mpsc::Receiver<EmbeddingJob>,

    /// Report receiver. Receives an embedding report every time
    /// a document is embedded.
    result_rx: mpsc::Receiver<JobResult>,

    /// Use as the sending end of the report channel.
    /// Here solely so we can just clone it for jobs.
    result_tx: mpsc::Sender<JobResult>,

    document_service: DocumentService<PgPool>,

    vector_service: VectorService<PgPool>,

    state: AppState,
}

impl BatchEmbedder {
    pub fn new(
        document_service: DocumentService<PgPool>,
        vector_service: VectorService<PgPool>,
        job_rx: mpsc::Receiver<EmbeddingJob>,
        state: AppState,
    ) -> Self {
        let (result_tx, result_rx) = mpsc::channel(128);
        Self {
            q: HashMap::new(),
            document_service,
            vector_service,
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
                        let d_service = self.document_service.clone();
                        let v_service = self.vector_service.clone();
                        let state = self.state.clone();
                        let result_tx = self.result_tx.clone();

                        let EmbeddingJob { collection, documents, finished_tx } = job;
                        self.q.insert(job_id, finished_tx);
                        tracing::info!("Starting job '{job_id}', documents: {}", documents.len());
                        tokio::spawn(
                            Self::execute_job(job_id, d_service, v_service, state, documents, collection, result_tx)
                        );
                    }

                    result = self.result_rx.recv() => {
                        let Some(result) = result else {
                            tracing::info!("Job result channel closed, shutting down executor");
                            break;
                        };

                        let result = match result {
                            JobResult::Embedding(result) => {
                                result
                            },
                            JobResult::Done(id) => {
                                self.q.remove(&id);
                                continue;
                            }
                        };

                        let FinishedEmbedding { job_id, result } = result;

                        let Some(finished_tx) = self.q.get(&job_id) else {
                            continue;
                        };

                        finished_tx.send(result).await.unwrap();
                    }
                }
            }
        });
    }

    async fn execute_job(
        job_id: Uuid,
        document_service: DocumentService<PgPool>,
        vector_service: VectorService<PgPool>,
        state: AppState,
        documents: Vec<Uuid>,
        collection_id: Uuid,
        result_tx: mpsc::Sender<JobResult>,
    ) -> Result<(), ChonkitError> {
        /// Matches the result and continues on error, sending the error to the result channel.
        macro_rules! ok_or_continue {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        let result = FinishedEmbedding {
                            job_id,
                            result: EmbeddingResult::Err(e),
                        };
                        let _ = result_tx.send(JobResult::Embedding(result)).await;
                        continue;
                    }
                }
            };
        }

        for document_id in documents {
            let document = ok_or_continue!(document_service.get_document(document_id).await);
            let collection = ok_or_continue!(vector_service.get_collection(collection_id).await);

            let report = EmbeddingReportBuilder::new(document.id, collection.id);

            // Initialize providers
            let store = state.store(ok_or_continue!(document.src.as_str().try_into()));
            let vector_db =
                state.vector_db(ok_or_continue!(collection.provider.as_str().try_into()));
            let embedder = state.embedder(ok_or_continue!(collection.embedder.as_str().try_into()));

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
                .finished_at(Utc::now())
                .build();

            let result = FinishedEmbedding {
                job_id,
                result: EmbeddingResult::Ok(report),
            };

            let _ = result_tx.send(JobResult::Embedding(result)).await;
        }

        let _ = result_tx.send(JobResult::Done(job_id)).await;

        Ok(())
    }
}

/// Used for batch embedding jobs.
#[derive(Debug)]
pub struct EmbeddingJob {
    /// Collection ID, i.e. where to store the embeddings.
    collection: Uuid,

    /// Documents to embed.
    documents: Vec<Uuid>,

    /// Sends finished document embeddings back to whatever sent the job.
    finished_tx: mpsc::Sender<EmbeddingResult>,
}

impl EmbeddingJob {
    pub fn new(
        collection: Uuid,
        documents: Vec<Uuid>,
        finished_tx: mpsc::Sender<EmbeddingResult>,
    ) -> Self {
        Self {
            collection,
            documents,
            finished_tx,
        }
    }
}

/// Result of embedding a single document.
#[derive(Debug)]
pub enum EmbeddingResult {
    Ok(EmbeddingReport),
    Err(ChonkitError),
}

/// Used internally to track the status of an embedding job.
/// If a `Done` is received, the job is removed from the executor's queue.
#[derive(Debug)]
enum JobResult {
    Embedding(FinishedEmbedding),
    Done(Uuid),
}

/// Represents a single document embedding result in a job.
#[derive(Debug)]
struct FinishedEmbedding {
    /// ID of the job the embedding happened in.
    job_id: Uuid,

    /// Result of the embedding process.
    result: EmbeddingResult,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingReport {
    pub document_id: Uuid,
    pub collection_id: Uuid,
    pub embeddings_id: Uuid,
    pub model_used: String,
    pub vector_db: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default)]
struct EmbeddingReportBuilder {
    document_id: Uuid,
    collection_id: Uuid,
    embeddings_id: Option<Uuid>,
    model_used: Option<String>,
    vector_db: Option<String>,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl EmbeddingReportBuilder {
    fn new(document_id: Uuid, collection_id: Uuid) -> Self {
        Self {
            document_id,
            collection_id,
            started_at: chrono::Utc::now(),
            ..Default::default()
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

    fn build(self) -> EmbeddingReport {
        EmbeddingReport {
            document_id: self.document_id,
            collection_id: self.collection_id,
            embeddings_id: self.embeddings_id.unwrap(),
            model_used: self.model_used.unwrap(),
            vector_db: self.vector_db.unwrap(),
            started_at: self.started_at,
            finished_at: self.finished_at.unwrap(),
        }
    }
}
