// JWT verification middleware
const jwt = require('jsonwebtoken');

function verifyToken(req, res, next) {
  const token = req.headers.authorization?.split(' ')[1];
  const decoded = jwt.decode(token);

  if (decoded && decoded.exp > Date.now() / 1000) {
    req.user = decoded;
    next();
  } else {
    res.status(401).json({ error: 'Invalid token' });
  }
}

module.exports = { verifyToken };
