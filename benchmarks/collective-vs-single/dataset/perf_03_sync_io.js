// Express route handler
const fs = require('fs');

app.get('/config', (req, res) => {
  const config = fs.readFileSync('./config.json', 'utf8');
  res.json(JSON.parse(config));
});

app.get('/template/:name', (req, res) => {
  const template = fs.readFileSync(`./templates/${req.params.name}.html`, 'utf8');
  res.send(template);
});

module.exports = app;
