// XML import service
const libxmljs = require('libxmljs');

function parseXmlData(req, res) {
  const xmlString = req.body.xml;

  try {
    const doc = libxmljs.parseXmlString(xmlString, { noent: true, dtdload: true });
    const data = extractData(doc);
    res.json(data);
  } catch (error) {
    res.status(400).json({ error: 'Invalid XML' });
  }
}

function extractData(doc) {
  return { root: doc.root().name() };
}

module.exports = { parseXmlData };
