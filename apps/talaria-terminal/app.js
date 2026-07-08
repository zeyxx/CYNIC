const feed = document.getElementById('feed');
const blockHeightEl = document.getElementById('block-height');

// Starting block height
let currentBlock = 849021;

// Simulated events for the "Unipersonal Blockchain"
const events = [
    "[CYNIC] Juge On-Chain déployé. Status: Active",
    "[TALARIA] Compilation de l'interface réussie.",
    "[MEMPOOL] Détection d'une intention de Fractal (Opt-In)",
    "[FUTARCHY] Proposition #42 soumise par $ASDFASDFA",
    "[VAULT] Frais de roulement routés : 14.5 USDC",
    "[ORACLE] Compression d'état: Proof-of-Build générée",
    "[CORE] Syncing with Solana mainnet-beta...",
    "[E-SCORE] Wallet 7Wg... verified (Score: 94.2)",
    "[GASdf] Transaction sponsorisée relayée (0.00 SOL)",
    "[LEGENDS] Invitation envoyée à la Communauté #04"
];

// Generate a random mock transaction hash
function generateHash() {
    const chars = 'abcdef0123456789';
    let hash = '0x';
    for (let i = 0; i < 16; i++) {
        hash += chars[Math.floor(Math.random() * chars.length)];
    }
    return hash + '...';
}

// Format current time as HH:MM:SS:MS
function getCurrentTime() {
    const now = new Date();
    return `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}.${now.getMilliseconds().toString().padStart(3, '0')}`;
}

// Create and append a new feed item
function addFeedItem(content) {
    const item = document.createElement('div');
    item.className = 'feed-item';
    
    item.innerHTML = `
        <div class="item-time">[${getCurrentTime()}]</div>
        <div class="item-content">${content}</div>
        <div class="item-hash">${generateHash()}</div>
    `;

    // Insert at the top
    feed.insertBefore(item, feed.firstChild);

    // Keep only last 50 items to prevent DOM bloat
    if (feed.children.length > 50) {
        feed.removeChild(feed.lastChild);
    }
}

// Update the block height occasionally
function updateBlockHeight() {
    currentBlock += 1;
    blockHeightEl.textContent = currentBlock;
}

// Simulating the live feed
function simulateStream() {
    // Pick a random event
    const randomEvent = events[Math.floor(Math.random() * events.length)];
    addFeedItem(randomEvent);

    // 10% chance to update block height when an event happens
    if (Math.random() > 0.9) {
        updateBlockHeight();
    }

    // Schedule next event (random interval between 400ms and 2500ms)
    const nextInterval = Math.floor(Math.random() * 2100) + 400;
    setTimeout(simulateStream, nextInterval);
}

// Init
document.addEventListener('DOMContentLoaded', () => {
    addFeedItem("[SYSTEM] Initialisation de TALARIA OS...");
    addFeedItem("[SYSTEM] Connexion au CYNIC Protocol établie.");
    
    setTimeout(simulateStream, 1500);
});
