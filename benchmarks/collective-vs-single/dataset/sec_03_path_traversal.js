// File download handler
const fs = require('fs');

function downloadFile(req, res) {
  const filePath = './uploads/' + req.params.filename;
  const content = fs.readFileSync(filePath);
  res.send(content);
}

module.exports = { downloadFile };
