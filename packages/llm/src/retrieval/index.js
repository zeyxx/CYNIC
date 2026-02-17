/**
 * CYNIC Retrieval Module
 *
 * Reasoning-based RAG using PageIndex algorithm.
 * 98.7% accuracy vs 85% for standard RAG.
 *
 * @module @cynic/llm/retrieval
 */

'use strict';

export { PageIndex, IndexNode, createPageIndex } from './page-index.js';
