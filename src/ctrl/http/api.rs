#[rustfmt::skip]
use super::router::{
    // Documents
    __path_list_documents,
    __path_get_document,
    __path_delete_document,
    __path_upload_documents,
    __path_chunk_preview,
    __path_parse_preview,
    __path_update_chunk_config,
    __path_update_parse_config,
    __path_sync,
    // Vectors
    __path_list_collections,
    __path_list_embedding_models,
    __path_get_collection,
    __path_create_collection,
    __path_embed,
    __path_search, 
};
use crate::{
    core::{
        chunk::{ChunkBaseConfig, Chunker, SlidingWindow, SnappingWindow},
        document::parser::ParseConfig,
        model::{
            collection::{Collection, Embedding, VectorCollection},
            document::{Document, DocumentConfig},
            Pagination,
        },
        service::document::dto::ChunkPreviewPayload,
    },
    ctrl::http::dto::{CreateCollectionPayload, SearchPayload, UploadResult},
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // Documents
        list_documents,
        get_document,
        delete_document,
        upload_documents,
        chunk_preview,
        parse_preview,
        update_chunk_config,
        update_parse_config,
        sync,
        // Vectors
        list_collections,
        get_collection,
        create_collection,
        list_embedding_models,
        embed,
        search,
    ),
    components(schemas(
        Pagination,
        Document,
        DocumentConfig,
        UploadResult,
        Chunker,
        SlidingWindow,
        SnappingWindow,
        ChunkBaseConfig,
        ChunkPreviewPayload,
        ParseConfig,
        CreateCollectionPayload,
        SearchPayload,
        Embedding,
        Collection,
        VectorCollection,
    ))
)]
pub struct ApiDoc;
