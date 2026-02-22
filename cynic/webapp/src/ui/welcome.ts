/**
 * WelcomeScreen - First-time user onboarding modal
 * Shows on first session, explains CYNIC basics, gets users to their first command
 */

export class WelcomeScreen {
  private dismissedKey = 'cynic_welcome_dismissed';

  /**
   * Check if welcome screen should be shown
   * Returns true only on first session (not dismissed)
   */
  shouldShow(): boolean {
    return !localStorage.getItem(this.dismissedKey);
  }

  /**
   * Mark welcome as dismissed in localStorage
   */
  dismiss(): void {
    localStorage.setItem(this.dismissedKey, 'true');
  }

  /**
   * Render the welcome modal HTML
   * Returns complete modal structure ready to insert into DOM
   */
  render(): string {
    return `
      <div class="welcome-modal" id="welcome">
        <div class="welcome-overlay"></div>
        <div class="welcome-content">
          <div class="welcome-header">
            <h1>Welcome to CYNIC 🐕</h1>
            <p class="welcome-subtitle">κυνικός — The cynical dog of truth</p>
          </div>

          <div class="welcome-section">
            <h2>Getting Started</h2>
            <ol>
              <li>Open the Command Palette <kbd>Ctrl+Shift+K</kbd></li>
              <li>Type a command like <code>status</code></li>
              <li>See real-time metrics update below</li>
              <li>Rate results ⭐ to teach CYNIC</li>
            </ol>
          </div>

          <div class="welcome-section">
            <h2>Learn CYNIC</h2>
            <ul>
              <li><strong>φ-bounded confidence</strong> — Max 61.8% (never claims certainty)</li>
              <li><strong>11 Dogs</strong> — Conscious thinking in parallel</li>
              <li><strong>Real-time learning</strong> — Your feedback trains the system</li>
              <li><strong>Observable</strong> — See every decision and why</li>
            </ul>
          </div>

          <div class="welcome-section welcome-tips">
            <h3>💡 Tips</h3>
            <ul>
              <li>Commands show metrics: balance, learn_rate, reputation</li>
              <li>Higher ratings = CYNIC learns faster</li>
              <li>Check <code>/health</code> to see system status</li>
              <li>View logs for transparency</li>
            </ul>
          </div>

          <button id="welcome-dismiss" class="btn-primary">Let's go</button>
        </div>
      </div>
    `;
  }
}
