// State variables
let activeTab = 'models';
let runningModel = null;
let statusPollingInterval = null;

// DOM Elements
const navItems = document.querySelectorAll('.nav-item');
const viewPanels = document.querySelectorAll('.view-panel');
const viewTitle = document.getElementById('view-title');
const viewDescription = document.getElementById('view-description');
const activeModelInfo = document.getElementById('active-model-info');
const activeModelName = document.querySelector('.active-model-name');
const statusDot = document.querySelector('.status-dot');
const statusText = document.querySelector('.status-text');
const toastEl = document.getElementById('toast');

// Views DOM
const modelsGrid = document.getElementById('models-grid');
const btnScanModels = document.getElementById('btn-scan-models');
const chatMessages = document.getElementById('chat-messages');
const chatInput = document.getElementById('chat-input');
const btnSendChat = document.getElementById('btn-send-chat');
const tempInput = document.getElementById('temperature');
const tempVal = document.getElementById('temp-val');
const tokensInput = document.getElementById('max-tokens');
const tokensVal = document.getElementById('tokens-val');
const systemPrompt = document.getElementById('system-prompt');
const chatApiKeyInput = document.getElementById('chat-api-key');
const keyBadge = document.getElementById('key-badge');

// Peer Attach View DOM
const btnProbePeers = document.getElementById('btn-probe-peers');
const btnApplyPeers = document.getElementById('btn-apply-peers');
const peersTableBody = document.getElementById('peers-table-body');

// Toast notifier
function showToast(message, type = 'success') {
  toastEl.innerText = message;
  toastEl.className = `toast show ${type}`;
  setTimeout(() => {
    toastEl.classList.remove('show');
  }, 4000);
}

// Tab Switching
navItems.forEach(item => {
  item.addEventListener('click', () => {
    const tabName = item.getAttribute('data-tab');
    switchTab(tabName);
  });
});

function switchTab(tabName) {
  activeTab = tabName;
  navItems.forEach(btn => {
    if (btn.getAttribute('data-tab') === tabName) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });

  viewPanels.forEach(panel => {
    if (panel.id === `view-${tabName}`) {
      panel.classList.add('active');
    } else {
      panel.classList.remove('active');
    }
  });

  // Update Headers
  if (tabName === 'models') {
    viewTitle.innerText = 'Models Vault';
    viewDescription.innerText = 'Sovereign GGUF model weight inventory and local controllers';
    loadVaultModels();
  } else if (tabName === 'playground') {
    viewTitle.innerText = 'Chat Playground';
    viewDescription.innerText = 'Interactive local inference and generation parameters';
  } else if (tabName === 'discovery') {
    viewTitle.innerText = 'Peer Attach & Discovery';
    viewDescription.innerText = 'Query and route to OpenAI-compatible peers on the network';
    loadPeers(false);
  } else if (tabName === 'system') {
    viewTitle.innerText = 'Host System Profile';
    viewDescription.innerText = 'Detailed hardware configuration and weight budgets';
    loadSystemInfo();
  }
}

// Load System Hardware Info
async function loadSystemInfo() {
  try {
    const res = await fetch('/api/hardware');
    const data = await res.json();
    
    document.getElementById('sys-cpu').innerText = `${data.cpu_threads} threads`;
    document.getElementById('sys-ram').innerText = `${Math.round(data.total_ram_bytes / (1024*1024*1024))} GB`;
    document.getElementById('sys-governor').innerText = data.cpu_governor || 'unknown';
    document.getElementById('sys-gpu-class').innerText = data.gpu.class || 'unknown';
    document.getElementById('sys-gpu-vendor').innerText = data.gpu.vendor_name || 'unknown';
    document.getElementById('sys-backends').innerText = data.backends.join(', ');
    document.getElementById('sys-budget').innerText = `${(data.recommended_max_weight_bytes / (1024*1024*1024)).toFixed(1)} GB`;
  } catch (err) {
    console.error('Failed to load system info:', err);
  }
}

// Load Vault Models
async function loadVaultModels() {
  modelsGrid.innerHTML = '<div class="loading-state">Scanning local model vault...</div>';
  try {
    const res = await fetch('/api/vault');
    const data = await res.json();
    
    if (data.status === 'error') {
      modelsGrid.innerHTML = `<div class="loading-state" style="color:var(--danger)">Error: ${data.message}</div>`;
      return;
    }

    if (!data.models || data.models.length === 0) {
      modelsGrid.innerHTML = '<div class="loading-state">No models found. Please download GGUF models or consolidate your vault.</div>';
      return;
    }

    modelsGrid.innerHTML = '';
    data.models.forEach(model => {
      const card = document.createElement('div');
      const isCurrent = runningModel && runningModel.alias === model.alias;
      card.className = `model-card ${isCurrent ? 'active' : ''}`;
      
      card.innerHTML = `
        <div class="model-card-header">
          <span class="model-alias">${model.alias}</span>
          <span class="model-type-badge">${model.model_exists ? 'Local GGUF' : 'Unavailable'}</span>
        </div>
        <div class="model-details">
          <div class="detail-row" style="margin-bottom: 6px;">
            <span class="detail-label">Port:</span>
            <input type="number" class="port-override" data-alias="${model.alias}" value="${model.port}" style="width: 75px; background: rgba(0,0,0,0.3); border: 1px solid var(--card-border); color: #fff; padding: 2px 6px; border-radius: 4px; text-align: center; font-size: 0.8rem;">
          </div>
          <div class="detail-row" style="margin-bottom: 6px;">
            <span class="detail-label">Context Limit:</span>
            <input type="number" class="ctx-override" data-alias="${model.alias}" value="${model.ctx_size}" style="width: 75px; background: rgba(0,0,0,0.3); border: 1px solid var(--card-border); color: #fff; padding: 2px 6px; border-radius: 4px; text-align: center; font-size: 0.8rem;">
          </div>
          <div class="detail-row">
            <span class="detail-label">Weight Size:</span>
            <span class="detail-value">${(model.size_bytes / (1024*1024*1024)).toFixed(2)} GB</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">GGUF Path:</span>
            <span class="detail-value" style="word-break: break-all; font-size: 0.75rem;">${model.model_path}</span>
          </div>
        </div>
        <div class="model-actions">
          ${isCurrent 
            ? `<button class="btn btn-danger btn-stop" style="flex:1" data-alias="${model.alias}">Stop Process</button>`
            : `<button class="btn btn-primary btn-start" style="flex:1" data-alias="${model.alias}" ${!model.model_exists ? 'disabled' : ''}>Start Process</button>`
          }
        </div>
      `;
      modelsGrid.appendChild(card);
    });

    // Attach click listeners
    document.querySelectorAll('.btn-start').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const alias = e.target.getAttribute('data-alias');
        const card = e.target.closest('.model-card');
        const portOverride = card.querySelector('.port-override').value;
        const ctxOverride = card.querySelector('.ctx-override').value;
        const supervisor = document.getElementById('supervisor-select').value;
        
        e.target.innerText = 'Starting...';
        e.target.disabled = true;
        updateStatusIndicator('loading', `Starting ${alias}...`);
        
        try {
          const startRes = await fetch(`/api/start?alias=${alias}&port=${portOverride}&ctx_size=${ctxOverride}&supervisor=${supervisor}`);
          const startReport = await startRes.json();
          
          if (startReport.ok) {
            showToast(`Model ${alias} started successfully!`, 'success');
            await checkStatus();
            switchTab('playground');
          } else {
            showToast(`Failed to start: ${startReport.message || 'Error'}`, 'error');
            await checkStatus();
            loadVaultModels();
          }
        } catch (err) {
          showToast(`Error starting model: ${err.message}`, 'error');
          await checkStatus();
          loadVaultModels();
        }
      });
    });

    document.querySelectorAll('.btn-stop').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const alias = e.target.getAttribute('data-alias');
        const supervisor = document.getElementById('supervisor-select').value;
        e.target.innerText = 'Stopping...';
        e.target.disabled = true;
        
        try {
          const stopRes = await fetch(`/api/stop?alias=${alias}&supervisor=${supervisor}`);
          const stopReport = await stopRes.json();
          
          if (stopReport.ok) {
            showToast(`Model stopped successfully.`, 'success');
          } else {
            showToast(`Failed to stop: ${stopReport.message || 'Error'}`, 'error');
          }
          await checkStatus();
          loadVaultModels();
        } catch (err) {
          showToast(`Error stopping model: ${err.message}`, 'error');
          await checkStatus();
          loadVaultModels();
        }
      });
    });

  } catch (err) {
    modelsGrid.innerHTML = `<div class="loading-state" style="color:var(--danger)">Error fetching vault: ${err.message}</div>`;
  }
}

// Check local status
async function checkStatus() {
  const supervisor = document.getElementById('supervisor-select')
    ? document.getElementById('supervisor-select').value
    : 'process';
  try {
    const res = await fetch(`/api/status?supervisor=${supervisor}`);
    const data = await res.json();
    
    if (data.state === 'Active') {
      runningModel = {
        alias: data.alias,
        endpoint: data.endpoint,
        pid: data.main_pid
      };
      
      activeModelInfo.style.display = 'flex';
      activeModelName.innerText = data.alias;
      updateStatusIndicator('online', `Running: ${data.alias} (${supervisor})`);
      
      // Enable Playground inputs
      chatInput.disabled = false;
      chatInput.placeholder = "Send message to local model...";
      btnSendChat.disabled = false;
    } else {
      runningModel = null;
      activeModelInfo.style.display = 'none';
      updateStatusIndicator('offline', `No active process (${supervisor})`);
      
      // Disable Playground inputs
      chatInput.disabled = true;
      chatInput.placeholder = "Start a model from the Vault to chat...";
      btnSendChat.disabled = true;
    }
  } catch (err) {
    updateStatusIndicator('offline', 'Dashboard server unreachable');
  }
}

function updateStatusIndicator(state, text) {
  statusDot.className = `status-dot ${state}`;
  statusText.innerText = text;
}

// Load external API Key info
async function checkApiKey() {
  try {
    const res = await fetch('/api/key');
    const data = await res.json();
    if (data.has_api_key) {
      keyBadge.innerText = 'Detected';
      keyBadge.className = 'badge detected';
    } else {
      keyBadge.innerText = 'None';
      keyBadge.className = 'badge';
    }
  } catch (e) {}
}

// Playground Settings Controls
tempInput.addEventListener('input', (e) => {
  tempVal.innerText = e.target.value;
});
tokensInput.addEventListener('input', (e) => {
  tokensVal.innerText = e.target.value;
});

// Chat Playground Flow
btnSendChat.addEventListener('click', sendChatMessage);
chatInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendChatMessage();
  }
});

async function sendChatMessage() {
  const text = chatInput.value.trim();
  if (!text || !runningModel) return;

  // Append user message
  appendMessage('user', text);
  chatInput.value = '';
  
  // Append assistant loading placeholder
  const botMessageId = appendMessage('system', 'Generating response...', 'thinking');

  try {
    const systemContent = systemPrompt.value.trim();
    const temp = parseFloat(tempInput.value);
    const tokens = parseInt(tokensInput.value);

    let endpoint = runningModel.endpoint;
    if (!endpoint.endsWith('/v1')) {
      endpoint = `${endpoint.replace(/\/$/, '')}/v1`;
    }
    
    const requestBody = {
      model: runningModel.alias,
      messages: [
        { role: 'system', content: systemContent },
        { role: 'user', content: text }
      ],
      temperature: temp,
      max_tokens: tokens,
      stream: false
    };

    const res = await fetch(`${endpoint}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(requestBody)
    });

    if (!res.ok) {
      throw new Error(`Inference returned status ${res.status}`);
    }

    const data = await res.json();
    const content = data.choices[0].message.content;
    
    // Update assistant response bubble
    updateMessageBubble(botMessageId, content);

  } catch (err) {
    updateMessageBubble(botMessageId, `Error communicating with model: ${err.message}. Make sure the process is fully initialized.`, 'error');
  }
}

function appendMessage(sender, text, extraClass = '') {
  const messageId = `msg-${Date.now()}`;
  const msgEl = document.createElement('div');
  msgEl.className = `message ${sender} ${extraClass}`;
  msgEl.id = messageId;
  
  msgEl.innerHTML = `
    <div class="msg-avatar">${sender === 'user' ? 'U' : 'C'}</div>
    <div class="msg-bubble">
      <p>${escapeHTML(text)}</p>
    </div>
  `;
  
  chatMessages.appendChild(msgEl);
  chatMessages.scrollTop = chatMessages.scrollHeight;
  return messageId;
}

function updateMessageBubble(id, text, stateClass = '') {
  const el = document.getElementById(id);
  if (el) {
    if (stateClass) {
      el.className = `message system ${stateClass}`;
    } else {
      el.classList.remove('thinking');
    }
    const bubble = el.querySelector('.msg-bubble p');
    if (bubble) {
      bubble.innerHTML = formatResponseText(text);
    }
  }
  chatMessages.scrollTop = chatMessages.scrollHeight;
}

function formatResponseText(text) {
  let formatted = escapeHTML(text)
    .replace(/\n/g, '<br>')
    .replace(/`([^`]+)`/g, '<code>$1</code>');
  return formatted;
}

function escapeHTML(str) {
  return str.replace(/[&<>'"]/g, 
    tag => ({
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      "'": '&#39;',
      '"': '&quot;'
    }[tag] || tag)
  );
}

// Peer Attach View
async function loadPeers(apply = false) {
  peersTableBody.innerHTML = '<tr><td colspan="4" class="empty-table">Probing local network and scanning ports...</td></tr>';
  try {
    const res = await fetch(`/api/discover?apply=${apply}`);
    const data = await res.json();
    
    if (!data.services || data.services.length === 0) {
      peersTableBody.innerHTML = '<tr><td colspan="4" class="empty-table">No OpenAI-compatible peers found on local ports (11434, 3000, 8080).</td></tr>';
      return;
    }

    peersTableBody.innerHTML = '';
    data.services.forEach(svc => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td><strong>${svc.name}</strong></td>
        <td><code style="font-size:0.8rem; background:rgba(255,255,255,0.05); padding:2px 6px; border-radius:4px;">${svc.endpoint}</code></td>
        <td><span class="badge detected">Online</span></td>
        <td>${svc.models && svc.models.length > 0 ? svc.models.join(', ') : 'none'}</td>
      `;
      peersTableBody.appendChild(tr);
    });

    if (apply) {
      showToast('Network routes successfully applied to inference.toml!', 'success');
    }
  } catch (err) {
    peersTableBody.innerHTML = `<tr><td colspan="4" class="empty-table" style="color:var(--danger)">Error: ${err.message}</td></tr>`;
  }
}

btnProbePeers.addEventListener('click', () => loadPeers(false));
btnApplyPeers.addEventListener('click', () => loadPeers(true));
btnScanModels.addEventListener('click', loadVaultModels);

document.getElementById('supervisor-select').addEventListener('change', () => {
  checkStatus();
  loadVaultModels();
});

// Initialization
async function init() {
  await checkStatus();
  await loadVaultModels();
  await checkApiKey();
  
  // Poll status every 4 seconds
  statusPollingInterval = setInterval(checkStatus, 4000);
}

document.addEventListener('DOMContentLoaded', init);
