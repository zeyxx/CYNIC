/**
 * Tests for error display functionality.
 *
 * Tests that:
 * - ErrorDisplay.show() creates and displays error elements
 * - Error messages are properly escaped
 * - Close button removes error
 * - Auto-dismiss works for non-critical errors
 * - Critical errors don't auto-dismiss
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ErrorDisplay } from '../src/ui/error-display';

describe('ErrorDisplay', () => {
  beforeEach(() => {
    // Clear any existing error messages
    ErrorDisplay.hideAll();
    // Ensure we have a body element
    if (!document.body) {
      document.documentElement.appendChild(document.createElement('body'));
    }
  });

  afterEach(() => {
    // Clean up
    ErrorDisplay.hideAll();
  });

  it('should display error message', () => {
    ErrorDisplay.show({
      error: 'Test error message',
      code: '#TEST',
      type: 'TestError',
    });

    const errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeTruthy();
    expect(errorEl?.textContent).toContain('Test error message');
    expect(errorEl?.textContent).toContain('#TEST');
  });

  it('should escape HTML in error message', () => {
    ErrorDisplay.show({
      error: '<script>alert("xss")</script>',
      code: '#XSS',
      type: 'SecurityTest',
    });

    const messageEl = document.querySelector('.error-message-text');
    expect(messageEl?.textContent).toBe('<script>alert("xss")</script>');
    // Verify no actual script tag was created
    const scriptInError = document.querySelector('.error-message script');
    expect(scriptInError).toBeFalsy();
  });

  it('should remove error when close button clicked', () => {
    ErrorDisplay.show({
      error: 'Closeable error',
      code: '#CLOSE',
      type: 'CloseTest',
    });

    let errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeTruthy();

    const closeBtn = document.querySelector('.error-close') as HTMLButtonElement;
    closeBtn?.click();

    errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeFalsy();
  });

  it('should auto-dismiss non-critical errors', async () => {
    ErrorDisplay.show(
      {
        error: 'Auto-dismiss error',
        code: '#AUTO',
        type: 'AutoDismissTest',
      },
      100 // 100ms duration
    );

    let errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeTruthy();

    // Wait for auto-dismiss
    await new Promise((resolve) => setTimeout(resolve, 150));

    errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeFalsy();
  });

  it('should not auto-dismiss critical errors (with support)', async () => {
    ErrorDisplay.show(
      {
        error: 'Critical error. Please contact support.',
        code: '#CRIT',
        type: 'CriticalError',
      },
      100 // Even with short duration, should not dismiss
    );

    let errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeTruthy();

    // Wait longer than the auto-dismiss duration
    await new Promise((resolve) => setTimeout(resolve, 200));

    // Should still be there because it mentions "support"
    errorEl = document.querySelector('.error-message');
    expect(errorEl).toBeTruthy();
  });

  it('should hide all errors', () => {
    // Create multiple errors
    ErrorDisplay.show(
      { error: 'Error 1', code: '#E1', type: 'Test1' },
      -1 // No auto-dismiss
    );
    ErrorDisplay.show(
      { error: 'Error 2', code: '#E2', type: 'Test2' },
      -1
    );

    let errors = document.querySelectorAll('.error-message');
    expect(errors.length).toBe(2);

    // Hide all
    ErrorDisplay.hideAll();

    errors = document.querySelectorAll('.error-message');
    expect(errors.length).toBe(0);
  });

  it('should display multiple sequential errors', () => {
    ErrorDisplay.show(
      { error: 'Error 1', code: '#E1', type: 'Test1' },
      -1
    );
    ErrorDisplay.show(
      { error: 'Error 2', code: '#E2', type: 'Test2' },
      -1
    );

    const errors = document.querySelectorAll('.error-message');
    expect(errors.length).toBe(2);
  });
});
