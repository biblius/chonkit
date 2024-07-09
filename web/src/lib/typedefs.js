export default {};
/**
 * Embedding payload.
 * @typedef {Object} EmbeddingPayload
 * @property {number} id - Document ID.
 * @property {string} collection - Vector collection.
 * @property {SlidingWindowInput | SnappingWindowInput | RecursiveInput } input - Chunk configuration.
 */

/**
 * Chunking configuration base.
 * @typedef {Object} ChunkConfig
 * @property {number} size - Chunk base.
 * @property {number} overlap - Chunk overlap, based on the chunker in question.
 */

/**
 * Chunking configuration payload.
 * @typedef {Object} SlidingWindowInput
 * @property {ChunkConfig} config - Chunk configuration.
 */

/**
 * Chunking configuration payload.
 * @typedef {Object} SnappingWindowInput
 * @property {ChunkConfig} config - Chunk configuration.
 * @property {string[]} skipF - Lookahead patterns for determining sentence stops.
 * @property {string[]} skipB - Lookahead patterns for determining sentence stops.
 */

/**
 * Chunking configuration payload.
 * @typedef {Object} RecursiveInput
 * @property {ChunkConfig} config - Chunk configuration.
 * @property {string[]} delimiters - Delimiters to use for splitting.
 */
