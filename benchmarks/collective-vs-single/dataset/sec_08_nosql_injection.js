// MongoDB user lookup
const { MongoClient } = require('mongodb');

async function findUser(req, res) {
  const { username, password } = req.body;
  const db = await MongoClient.connect(process.env.MONGO_URI);
  const user = await db.collection('users').findOne({ username, password });
  res.json(user);
}

module.exports = { findUser };
