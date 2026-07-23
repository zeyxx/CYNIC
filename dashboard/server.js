const http = require('http');
const fs = require('fs');
const path = require('path');
const { exec } = require('child_process');

const PORT = 3333;
const STATIC_DIR = path.join(__dirname, 'public');
const CYNIC_BIN = path.join(__dirname, '..', 'target', 'debug', 'cynic');

// Helper to execute CLI and return json
function runCynic(args, res, envOverrides = {}) {
  const cmd = `"${CYNIC_BIN}" ${args}`;
  const env = {
    ...process.env,
    CYNIC_ALLOW_LOCAL_DEV_BUILD: '1',
    CARGO_INCREMENTAL: '0',
    ...envOverrides
  };
  exec(cmd, { env }, (error, stdout, stderr) => {
    res.writeHead(200, { 
      'Content-Type': 'application/json',
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type'
    });
    if (error) {
      res.end(JSON.stringify({ status: 'error', message: stderr || error.message }));
      return;
    }
    res.end(stdout);
  });
}

const server = http.createServer((req, res) => {
  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  const parsedUrl = new URL(req.url, `http://${req.headers.host}`);
  const pathname = parsedUrl.pathname;

  if (pathname.startsWith('/api/')) {
    if (pathname === '/api/vault') {
      runCynic('runtime inference-vault --json', res);
    } else if (pathname === '/api/status') {
      runCynic('runtime inference-status --json', res);
    } else if (pathname === '/api/hardware') {
      runCynic('runtime inference-hardware --json', res);
    } else if (pathname === '/api/start') {
      const alias = parsedUrl.searchParams.get('alias') || 'auto';
      const ctxSize = parsedUrl.searchParams.get('ctx_size');
      const port = parsedUrl.searchParams.get('port');
      const supervisor = parsedUrl.searchParams.get('supervisor');
      const env = {};
      if (ctxSize) env.CYNIC_OVERRIDE_CTX_SIZE = ctxSize;
      if (port) env.CYNIC_OVERRIDE_PORT = port;
      if (supervisor) env.CYNIC_INFERENCE_SUPERVISOR = supervisor;
      runCynic(`runtime inference-start ${alias} --json`, res, env);
    } else if (pathname === '/api/stop') {
      const alias = parsedUrl.searchParams.get('alias');
      const supervisor = parsedUrl.searchParams.get('supervisor');
      const arg = alias ? alias : '';
      const env = {};
      if (supervisor) env.CYNIC_INFERENCE_SUPERVISOR = supervisor;
      runCynic(`runtime inference-stop ${arg} --json`, res, env);
    } else if (pathname === '/api/discover') {
      const apply = parsedUrl.searchParams.get('apply') === 'true';
      runCynic(`runtime inference-discover --json ${apply ? '--apply' : ''}`, res);
    } else if (pathname === '/api/key') {
      // Check if api key exists
      const principalKey = path.join(require('os').homedir(), '.config/cynic/llama-api-key');
      const systemKey = '/etc/cynic/llama-api-key';
      const hasKey = fs.existsSync(principalKey) || fs.existsSync(systemKey);
      let apiKey = '';
      if (fs.existsSync(principalKey)) {
        try { apiKey = fs.readFileSync(principalKey, 'utf8').trim(); } catch(e){}
      } else if (fs.existsSync(systemKey)) {
        try { apiKey = fs.readFileSync(systemKey, 'utf8').trim(); } catch(e){}
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ has_api_key: hasKey, api_key: apiKey }));
    } else {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Not Found' }));
    }
    return;
  }

  // Serve static files
  let filePath = path.join(STATIC_DIR, pathname === '/' ? 'index.html' : pathname);
  
  // Guard directory traversal
  if (!filePath.startsWith(STATIC_DIR)) {
    res.writeHead(403, { 'Content-Type': 'text/plain' });
    res.end('Forbidden');
    return;
  }

  const ext = path.extname(filePath);
  let contentType = 'text/html';
  if (ext === '.css') contentType = 'text/css';
  else if (ext === '.js') contentType = 'text/javascript';
  else if (ext === '.png') contentType = 'image/png';
  else if (ext === '.svg') contentType = 'image/svg+xml';

  fs.readFile(filePath, (err, content) => {
    if (err) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end('404 Not Found');
    } else {
      res.writeHead(200, { 'Content-Type': contentType });
      res.end(content);
    }
  });
});

server.listen(PORT, () => {
  console.log(`CYNIC Dashboard Server running at http://localhost:${PORT}`);
});
