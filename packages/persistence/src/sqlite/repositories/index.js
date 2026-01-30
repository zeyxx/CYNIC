/**
 * SQLite Repositories Index
 *
 * Exports all SQLite-compatible repositories for offline/edge deployment.
 *
 * @module @cynic/persistence/sqlite/repositories
 */

'use strict';

import { SQLiteJudgmentRepository } from './judgments.js';
import { SQLitePatternRepository } from './patterns.js';
import { SQLiteUserRepository } from './users.js';

export { SQLiteJudgmentRepository, SQLitePatternRepository, SQLiteUserRepository };

export default {
  SQLiteJudgmentRepository,
  SQLitePatternRepository,
  SQLiteUserRepository,
};
