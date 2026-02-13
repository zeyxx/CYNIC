import { getPool } from '@cynic/persistence';

const pool = getPool();
console.log('Connection string:', pool.connectionString ? pool.connectionString.substring(0, 40) + '...' : 'NULL');
console.log('Pool exists:', !!pool.pool);

const cs = pool._breaker?.getState();
console.log('Circuit breaker state:', cs?.state);
console.log('Circuit breaker failures:', cs?.consecutiveFailures);
console.log('Circuit breaker openings:', cs?.consecutiveOpenings);

// Try a query
try {
  const result = await pool.query('SELECT 1 as test');
  console.log('Query OK:', result.rows[0]);
  console.log('CB after success:', pool._breaker?.getState()?.state);
} catch (err) {
  console.log('Query FAILED:', err.message);
}

process.exit(0);
