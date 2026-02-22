/**
 * User-friendly error display for the CYNIC webapp.
 *
 * Shows errors to users in a friendly format:
 * - No raw Python exceptions
 * - Clear explanation of what happened
 * - Suggested action (retry, verify input, contact support)
 * - Error code for support reference
 *
 * Usage:
 *   ErrorDisplay.show({
 *     error: "Database connection issue...",
 *     code: "#DDEB",
 *     type: "DatabaseError"
 *   });
 */

export interface ErrorResponse {
  error: string;
  code: string;
  type: string;
}

export class ErrorDisplay {
  /**
   * Display an error message to the user.
   *
   * @param errorResponse - Error response from server
   * @param duration - How long to show error (ms). Infinite if -1 or contains "support".
   */
  static show(errorResponse: ErrorResponse, duration: number = 5000): void {
    const errorEl = document.createElement("div");
    errorEl.className = "error-message";

    // Build the error container with safe DOM manipulation
    errorEl.innerHTML = `
      <div class="error-content">
        <div class="error-icon">⚠️</div>
        <div class="error-body">
          <div class="error-title">Error (${errorResponse.code})</div>
          <div class="error-message-text"></div>
        </div>
        <button class="error-close" aria-label="Close">✕</button>
      </div>
    `;

    // Set error message text content (safe from XSS)
    const messageEl = errorEl.querySelector(".error-message-text");
    if (messageEl) {
      messageEl.textContent = errorResponse.error;
    }

    document.body.appendChild(errorEl);

    // Close button handler
    const closeBtn = errorEl.querySelector(".error-close");
    closeBtn?.addEventListener("click", () => {
      errorEl.remove();
    });

    // Auto-dismiss after duration
    // But keep it longer if it mentions "support"
    if (errorResponse.error.includes("support") || errorResponse.error.includes("contact")) {
      // Don't auto-dismiss for critical errors
      return;
    }

    if (duration > 0) {
      setTimeout(() => {
        if (errorEl.parentNode) {
          errorEl.remove();
        }
      }, duration);
    }
  }

  /**
   * Hide all visible error messages.
   */
  static hideAll(): void {
    const errors = document.querySelectorAll(".error-message");
    errors.forEach((el) => el.remove());
  }
}
