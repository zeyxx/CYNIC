/**
 * Learning Module
 *
 * Exports SONA and related learning components.
 *
 * @module @cynic/node/learning
 */

'use strict';

export {
  SONA,
  createSONA,
  SONA_CONFIG,
} from './sona.js';

export default {
  SONA,
  createSONA,
  SONA_CONFIG,
};

// Re-export from sona for convenience
import { SONA, createSONA, SONA_CONFIG } from './sona.js';
