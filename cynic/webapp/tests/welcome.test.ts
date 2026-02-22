import { describe, test, expect, beforeEach, afterEach } from 'vitest';
import { WelcomeScreen } from '../src/ui/welcome';

describe('WelcomeScreen', () => {
  beforeEach(() => {
    // Clear localStorage before each test
    localStorage.clear();
    // Clear DOM
    document.body.innerHTML = '';
  });

  afterEach(() => {
    localStorage.clear();
    document.body.innerHTML = '';
  });

  test('should show on first session', () => {
    const welcome = new WelcomeScreen();
    expect(welcome.shouldShow()).toBe(true);
  });

  test('should not show after dismissed', () => {
    const welcome = new WelcomeScreen();
    welcome.dismiss();
    expect(welcome.shouldShow()).toBe(false);
  });

  test('should render with title and instructions', () => {
    const welcome = new WelcomeScreen();
    const html = welcome.render();
    expect(html).toContain('Welcome to CYNIC');
    expect(html).toContain('Getting Started');
  });

  test('should have dismiss button', () => {
    const welcome = new WelcomeScreen();
    const html = welcome.render();
    expect(html).toContain('welcome-dismiss');
    expect(html).toContain("Let's go");
  });

  test('localStorage persists dismissed state', () => {
    const w1 = new WelcomeScreen();
    w1.dismiss();

    const w2 = new WelcomeScreen();
    expect(w2.shouldShow()).toBe(false);
  });
});
