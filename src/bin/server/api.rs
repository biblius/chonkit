#[rustfmt::skip]
use super::router::{
    // App config
    __path_app_config,
    // Documents
    __path_list_documents,
    __path_list_documents_display,
    __path_get_document,
    __path_delete_document,
    __path_upload_documents,
    __path_chunk_preview,
    __path_parse_preview,
    __path_update_document_config,
    __path_sync,
    // Vectors
    __path_list_collections,
    __path_list_embedding_models,
    __path_get_collection,
    __path_create_collection,
    __path_delete_collection,
    __path_list_embedded_documents,
    __path_embed,
    __path_batch_embed,
    __path_search, 
    __path_count_embeddings,
    __path_delete_embeddings,
    __path_list_collections_display,
    __path_collection_display
};
use crate::dto::{
    ChunkPreviewPayload, CreateCollectionPayload, EmbeddingBatchPayload, EmbeddingSinglePayload,
    ListDocumentsPayload, ListEmbeddingsPayload, SearchPayload, UploadResult,
};
use chonkit::{
    app::state::AppConfig,
    core::{
        chunk::{
            ChunkBaseConfig, Chunker, DistanceFn, SemanticWindow, SemanticWindowConfig,
            SlidingWindow, SnappingWindow,
        },
        document::parser::ParseConfig,
        model::{
            collection::{
                Collection, CollectionDisplay, CollectionShort, Embedding, VectorCollection,
            },
            document::{Document, DocumentConfig, DocumentDisplay, DocumentShort},
            List, Pagination, PaginationSort,
        },
    },
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // App config
        app_config,
        // Documents
        list_documents,
        list_documents_display,
        get_document,
        delete_document,
        upload_documents,
        chunk_preview,
        parse_preview,
        update_document_config,
        sync,
        // Vectors
        list_collections,
        get_collection,
        create_collection,
        delete_collection,
        list_embedding_models,
        list_embedded_documents,
        embed,
        batch_embed,
        search,
        delete_embeddings,
        count_embeddings,
        list_collections_display,
        collection_display,
    ),
    components(schemas(
        List<Collection>,
        List<Document>,
        List<DocumentDisplay>,
        Pagination,
        PaginationSort,
        Document,
        DocumentConfig,
        UploadResult,
        Chunker,
        SlidingWindow,
        SnappingWindow,
        SemanticWindow,
        SemanticWindowConfig,
        DistanceFn,
        ChunkBaseConfig,
        ChunkPreviewPayload,
        ParseConfig,
        CreateCollectionPayload,
        SearchPayload,
        Embedding,
        Collection,
        VectorCollection,
        AppConfig,
        EmbeddingBatchPayload,
        EmbeddingSinglePayload,
        ListEmbeddingsPayload,
        ListDocumentsPayload,
        // Display
        DocumentDisplay,
        DocumentShort,
        CollectionDisplay,
        CollectionShort,
    ))
)]
pub struct ApiDoc;
