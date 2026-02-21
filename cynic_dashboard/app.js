// CYNIC Dashboard - Vanilla JS
// Simple, maintainable HTML/CSS/JS stack

const DOG_NAMES = [
    'ANALYST', 'ARCHITECT', 'CARTOGRAPHER', 'CYNIC', 'DEPLOYER',
    'GUARDIAN', 'JANITOR', 'ORACLE', 'SAGE', 'SCHOLAR', 'SCOUT'
];

const DOG_COLORS = {
    ANALYST: '#8B5CF6',
    ARCHITECT: '#3B82F6',
    CARTOGRAPHER: '#06B6D4',
    CYNIC: '#EF4444',
    DEPLOYER: '#10B981',
    GUARDIAN: '#F59E0B',
    JANITOR: '#6366F1',
    ORACLE: '#EC4899',
    SAGE: '#F97316',
    SCHOLAR: '#8B5CF6',
    SCOUT: '#14B8A6'
};

let state = {
    connected: false,
    lastJudgment: null,
    eventCount: 0,
    judgmentCount: 0
};

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    setupTabNavigation();
    connectWebSocket();
});

// Tab Navigation
function setupTabNavigation() {
    const tabBtns = document.querySelectorAll('.tab-btn');
    const tabPanes = document.querySelectorAll('.tab-pane');

    tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabName = btn.dataset.tab;

            // Remove active class from all
            tabBtns.forEach(b => b.classList.remove('active'));
            tabPanes.forEach(p => p.classList.remove('active'));

            // Add active class to clicked tab
            btn.classList.add('active');
            const pane = document.getElementById(tabName);
            if (pane) pane.classList.add('active');
        });
    });
}

// WebSocket Connection
function connectWebSocket() {
    const wsUrl = 'ws://localhost:8000/ws/stream';
    const ws = new WebSocket(wsUrl);

    ws.onopen = () => {
        console.log('Connected to CYNIC kernel');
        state.connected = true;
        updateConnectionStatus();
    };

    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            state.eventCount++;

            if (data.type === 'JUDGMENT_CREATED' && data.payload) {
                state.lastJudgment = data.payload;
                state.judgmentCount++;
                updateAllViews();
            } else if (data.type === 'connected') {
                console.log('CYNIC kernel ready');
            } else if (data.type === 'ping') {
                ws.send(JSON.stringify({ type: 'pong' }));
            }

            updateStats();
        } catch (err) {
            console.error('WebSocket error:', err);
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        state.connected = false;
        updateConnectionStatus();
    };

    ws.onclose = () => {
        state.connected = false;
        updateConnectionStatus();
        setTimeout(connectWebSocket, 3000);
    };
}

// Update Connection Status
function updateConnectionStatus() {
    const statusEl = document.getElementById('status');
    if (state.connected) {
        statusEl.textContent = '🟢 Connected';
        statusEl.className = 'status connected';
    } else {
        statusEl.textContent = '🔴 Disconnected';
        statusEl.className = 'status disconnected';
    }
}

// Update All Views
function updateAllViews() {
    if (!state.lastJudgment) return;
    updateDogVoting();
    updateJudgment();
    updateStats();
}

// Dogs Voting Tab
function updateDogVoting() {
    const judgment = state.lastJudgment;
    if (!judgment.dogs_votes) return;

    const votes = judgment.dogs_votes;
    const scores = DOG_NAMES
        .map(name => ({ name, score: votes[name] || 0 }))
        .sort((a, b) => b.score - a.score);

    const html = scores.map(dog => {
        const percentage = (dog.score / 100) * 100;
        const color = DOG_COLORS[dog.name];
        return `<div class="dog-item">
            <div class="dog-name">${dog.name}</div>
            <div class="dog-score">Q: ${dog.score.toFixed(1)}</div>
            <div class="dog-bar">
                <div class="dog-bar-fill" style="width: ${percentage}%; background-color: ${color}; opacity: 0.8">
                    ${percentage > 5 ? percentage.toFixed(0) + '%' : ''}
                </div>
            </div>
            <div class="dog-percentage">${percentage.toFixed(0)}%</div>
        </div>`;
    }).join('');

    document.getElementById('dogs-list').innerHTML = html;

    const avg = scores.reduce((sum, d) => sum + d.score, 0) / scores.length;
    const active = scores.filter(d => d.score > 0).length;

    document.getElementById('avg-score').textContent = avg.toFixed(1);
    document.getElementById('active-dogs').textContent = active + '/11';
    document.getElementById('max-score').textContent = scores[0]?.score.toFixed(1) || '—';
}

// Judgment Tab
function updateJudgment() {
    const judgment = state.lastJudgment;
    if (!judgment) return;

    const verdictClass = (judgment.verdict || 'bark').toLowerCase();
    const emoji = {
        'howl': '🟢',
        'wag': '🟡',
        'growl': '🟠',
        'bark': '🔴'
    }[verdictClass] || '🔴';

    const confidencePercent = (judgment.confidence * 100).toFixed(1);

    let contentHtml = `<div class="verdict-banner ${verdictClass}">
        <div class="verdict-emoji">${emoji}</div>
        <div class="verdict-content">
            <h3>Verdict</h3>
            <div class="verdict-name">${judgment.verdict}</div>
        </div>
    </div>

    <div class="scores-grid">
        <div class="score-box">
            <div class="score-label">Q-Score</div>
            <div class="score-value">${judgment.q_score?.toFixed(1) || '—'}</div>
        </div>
        <div class="score-box">
            <div class="score-label">Confidence</div>
            <div class="score-value">${confidencePercent}%</div>
        </div>
    </div>`;

    if (judgment.reasoning) {
        contentHtml += `<div class="reasoning"><strong>Reasoning:</strong><br>${judgment.reasoning}</div>`;
    }

    document.getElementById('judgment-content').innerHTML = contentHtml;

    const metaHtml = `<div style="margin-bottom: 1rem">
        <strong>ID:</strong><br>
        <code style="font-size: 0.75rem">${judgment.id?.substring(0, 24) || '—'}...</code>
    </div>
    <div style="margin-bottom: 1rem">
        <strong>Source:</strong><br>
        ${judgment.source || 'unknown'}
    </div>
    <div>
        <strong>Cell ID:</strong><br>
        <code style="font-size: 0.75rem">${judgment.cell_id?.substring(0, 24) || '—'}...</code>
    </div>`;

    document.getElementById('judgment-meta').innerHTML = metaHtml;
    document.getElementById('stat-confidence').textContent = confidencePercent + '%';
}

// Update Stats Tab
function updateStats() {
    document.getElementById('stat-conn').textContent = state.connected ? 'Connected' : 'Disconnected';
    document.getElementById('stat-events').textContent = state.eventCount;
    document.getElementById('stat-judgments').textContent = state.judgmentCount;

    if (state.lastJudgment) {
        const ts = new Date();
        document.getElementById('stat-last-ts').textContent = ts.toLocaleTimeString();
    }
}

// Debug
window.debugDashboard = () => {
    console.log('State:', state);
    console.log('Last Judgment:', state.lastJudgment);
};
