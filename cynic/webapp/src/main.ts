/**
 * CYNIC Webapp - Main Entry Point
 * TypeScript vanilla application with esbuild
 */

// Import API clients to make them available globally
import { apiClient } from './api/client';
import { wsClient } from './api/ws';

// Export for global access in browser console
declare global {
  interface Window {
    CYNIC: {
      api: typeof apiClient;
      ws: typeof wsClient;
    };
  }
}

window.CYNIC = {
  api: apiClient,
  ws: wsClient,
};

console.log('*sniff* CYNIC Webapp initialized');

// Initialize the application
function initializeApp(): void {
  console.log('Initializing CYNIC control panel...');

  // Set up uptime counter
  const startTime = Date.now();
  setInterval(() => {
    const elapsed = Math.floor((Date.now() - startTime) / 1000);
    const hours = Math.floor(elapsed / 3600);
    const minutes = Math.floor((elapsed % 3600) / 60);
    const seconds = elapsed % 60;
    const uptime = `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;

    const uptimeEl = document.getElementById('stat-uptime');
    if (uptimeEl) {
      uptimeEl.textContent = uptime;
    }
  }, 1000);

  // Initialize command handlers
  const commands = document.querySelectorAll('.command-item');
  commands.forEach((cmd, index) => {
    cmd.addEventListener('click', () => {
      const title = (cmd as HTMLElement).querySelector('strong')?.textContent || 'Unknown';
      handleCommand(title);
    });
  });

  // Set initial stats
  updateDashboard();

  // Log initialization complete
  logOutput('CYNIC Webapp loaded successfully');
  logOutput('Commands available: Fetch Status, Learn Rate, Confidence Score, Memory Snapshot, Restart Organism');
}

function handleCommand(commandName: string): void {
  console.log(`Executing command: ${commandName}`);
  logOutput(`> ${commandName}`);

  // Simulate command execution based on type
  switch (commandName) {
    case 'Fetch Status':
      logOutput('Fetching organism status...');
      setTimeout(() => {
        logOutput('Status: All systems nominal');
        logOutput('Confidence: 58% (φ-bounded)');
        logOutput('Learn Rate: 0.618');
      }, 500);
      break;

    case 'Learn Rate':
      logOutput('Current learning rate: 0.618');
      logOutput('Theta (θ) parameters optimized');
      break;

    case 'Confidence Score':
      logOutput('Confidence Analysis:');
      logOutput('  φ-bound: 61.8% (theoretical max)');
      logOutput('  Current: 58%');
      logOutput('  Status: HEALTHY');
      break;

    case 'Memory Snapshot':
      logOutput('Memory state:');
      logOutput('  Entities: 1,247');
      logOutput('  Relations: 3,891');
      logOutput('  Tokens used: 2,156');
      break;

    case 'Restart Organism':
      logOutput('Restarting CYNIC kernel...');
      logOutput('⚠️  WARNING: This will reset the current session');
      break;

    default:
      logOutput(`Unknown command: ${commandName}`);
  }
}

function updateDashboard(): void {
  // Update confidence
  const confidenceEl = document.getElementById('stat-confidence');
  if (confidenceEl) {
    confidenceEl.textContent = '58%';
  }

  // Update learn rate
  const learnRateEl = document.getElementById('stat-learn-rate');
  if (learnRateEl) {
    learnRateEl.textContent = '0.618';
  }

  // Update memory stats
  const entitiesEl = document.getElementById('stat-entities');
  if (entitiesEl) {
    entitiesEl.textContent = '1,247';
  }

  const relationsEl = document.getElementById('stat-relations');
  if (relationsEl) {
    relationsEl.textContent = '3,891';
  }

  // Simulate latency measurement
  const latencyEl = document.getElementById('stat-latency');
  if (latencyEl) {
    const latency = Math.floor(Math.random() * 100) + 50;
    latencyEl.textContent = `${latency}ms`;
  }
}

function logOutput(message: string): void {
  const outputLog = document.getElementById('output-log');
  if (outputLog) {
    const timestamp = new Date().toLocaleTimeString();
    const line = `[${timestamp}] ${message}\n`;
    outputLog.textContent += line;
    // Auto-scroll to bottom
    const outputDiv = document.getElementById('output');
    if (outputDiv) {
      outputDiv.scrollTop = outputDiv.scrollHeight;
    }
  }
}

// Start the application when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeApp);
} else {
  initializeApp();
}

// Log that main.ts has been loaded and bundled
console.log('*tail wag* Bundle loaded successfully');
