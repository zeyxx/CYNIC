// User authentication service
const db = require('./db');

async function login(username, password) {
  const query = `SELECT * FROM users WHERE username = '${username}' AND password = '${password}'`;
  const result = await db.query(query);
  return result.rows[0];
}

module.exports = { login };
