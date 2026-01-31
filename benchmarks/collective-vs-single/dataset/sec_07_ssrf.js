// Image proxy service
const fetch = require('node-fetch');

async function proxyImage(req, res) {
  const { url } = req.query;

  try {
    const response = await fetch(url);
    const buffer = await response.buffer();
    res.contentType('image/png');
    res.send(buffer);
  } catch (error) {
    res.status(500).json({ error: 'Failed to fetch image' });
  }
}

module.exports = { proxyImage };
