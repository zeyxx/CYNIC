#!/usr/bin/env node
/**
 * Phase 16: Total Memory + Full Autonomy Integration Tests
 *
 * End-to-end integration tests for Phase 16 features:
 * - Memory retrieval → Autonomous capabilities → Dog behaviors
 * - Goal tracking → Task scheduling → Notification generation
 * - Self-correction loop
 *
 * "φ remembers and acts without being asked" - CYNIC
 *
 * @module @cynic/node/test/phase16-integration
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  AUTONOMOUS_CONSTANTS,
  DogGoalTypes,
  createAutonomousCapabilities,
} from '../src/agents/collective/autonomous.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ============================================================================
// MOCK INFRASTRUCTURE
// ============================================================================

function createMockRepositories() {
  const goals = [];
  const tasks = [];
  const notifications = [];
  const lessons = [];
  const decisions = [];

  return {
    goals: {
      create: async (data) => {
        const goal = { id: `goal-${goals.length + 1}`, ...data, progress: 0, status: 'active' };
        goals.push(goal);
        return goal;
      },
      findByUser: async (userId) => goals.filter(g => g.userId === userId),
      findById: async (id) => goals.find(g => g.id === id),
      updateProgress: async (id, progress) => {
        const goal = goals.find(g => g.id === id);
        if (goal) {
          goal.progress = Math.min(1, (goal.progress || 0) + progress);
          if (goal.progress >= 1) goal.status = 'completed';
        }
        return goal;
      },
      updateStatus: async (id, status) => {
        const goal = goals.find(g => g.id === id);
        if (goal) goal.status = status;
        return goal;
      },
      complete: async (id) => {
        const goal = goals.find(g => g.id === id);
        if (goal) goal.status = 'completed';
        return goal;
      },
      _goals: goals,
    },
    tasks: {
      create: async (data) => {
        const task = {
          id: `task-${tasks.length + 1}`,
          ...data,
          status: 'pending',
          createdAt: new Date(),
        };
        tasks.push(task);
        return task;
      },
      getPending: async (limit = 10) => tasks.filter(t => t.status === 'pending').slice(0, limit),
      findById: async (id) => tasks.find(t => t.id === id),
      claim: async (id) => {
        const task = tasks.find(t => t.id === id);
        if (task) task.status = 'running';
        return true;
      },
      complete: async (id, result) => {
        const task = tasks.find(t => t.id === id);
        if (task) {
          task.status = 'completed';
          task.result = result;
        }
        return task;
      },
      fail: async (id, error) => {
        const task = tasks.find(t => t.id === id);
        if (task) {
          task.status = 'failed';
          task.error = error;
        }
        return task;
      },
      _tasks: tasks,
    },
    notifications: {
      create: async (data) => {
        const notif = {
          id: `notif-${notifications.length + 1}`,
          ...data,
          delivered: false,
          createdAt: new Date(),
        };
        notifications.push(notif);
        return notif;
      },
      getPending: async (userId, limit) =>
        notifications.filter(n => n.userId === userId && !n.delivered).slice(0, limit),
      markDelivered: async (id) => {
        const notif = notifications.find(n => n.id === id);
        if (notif) notif.delivered = true;
        return notif;
      },
      _notifications: notifications,
    },
    memory: {
      checkForMistakes: async (userId, action) => ({
        warning: action.includes('danger') || action.includes('risky'),
        message: action.includes('danger') ? 'Previous similar mistake detected!' : null,
        lessons: action.includes('danger') ? [{ id: 'les-1', mistake: 'Similar mistake', correction: 'Do this instead' }] : [],
      }),
      rememberLesson: async (userId, lesson) => {
        const les = { id: `les-${lessons.length + 1}`, userId, ...lesson };
        lessons.push(les);
        return les;
      },
      rememberDecision: async (userId, decision) => {
        const dec = { id: `dec-${decisions.length + 1}`, userId, ...decision };
        decisions.push(dec);
        return dec;
      },
      _lessons: lessons,
      _decisions: decisions,
    },
  };
}

// ============================================================================
// INTEGRATION TEST: AUTONOMOUS CAPABILITIES LIFECYCLE
// ============================================================================

describe('Phase 16 Integration: Autonomous Capabilities Lifecycle', () => {
  let capabilities;
  let repos;

  beforeEach(() => {
    repos = createMockRepositories();
    capabilities = createAutonomousCapabilities(repos);
  });

  describe('Goal → Task → Notification Flow', () => {
    it('completes full autonomy cycle', async () => {
      const userId = 'user1';

      // 1. Track a goal
      const goal = await capabilities.trackGoal(userId, 'guardian', {
        type: 'security_audit',
        title: 'Security Audit',
        description: 'Audit the codebase for vulnerabilities',
      });

      assert.ok(goal, 'Goal should be created');
      assert.equal(goal.status, 'active');
      assert.equal(goal.progress, 0);

      // 2. Schedule related tasks
      const task1 = await capabilities.scheduleTask(userId, 'scan_dependencies', {
        goalId: goal.id,
        scope: 'full',
      });
      const task2 = await capabilities.scheduleTask(userId, 'check_secrets', {
        goalId: goal.id,
      });

      assert.ok(task1, 'Task 1 should be created');
      assert.ok(task2, 'Task 2 should be created');
      assert.equal(task1.status, 'pending');
      assert.equal(task2.status, 'pending');

      // 3. Complete tasks and update goal progress
      await repos.tasks.complete(task1.id, { vulnerabilities: [] });
      await capabilities.updateGoalProgress(userId, goal.id, 0.5);

      await repos.tasks.complete(task2.id, { secretsFound: 0 });
      await capabilities.updateGoalProgress(userId, goal.id, 0.5);

      // 4. Check goal completion
      const updatedGoal = await repos.goals.findById(goal.id);
      assert.equal(updatedGoal.progress, 1);
      assert.equal(updatedGoal.status, 'completed');

      // 5. Generate notification about completion
      const notification = await capabilities.notify(userId, {
        type: 'achievement',
        title: 'Security Audit Complete',
        message: 'All security checks passed!',
        importance: 0.7,
      });

      assert.ok(notification, 'Notification should be created');
      assert.equal(notification.notificationType, 'achievement');
    });

    it('tracks multiple goals independently', async () => {
      const userId = 'user1';

      const goal1 = await capabilities.trackGoal(userId, 'guardian', {
        type: 'security_audit',
        title: 'Goal 1',
      });
      const goal2 = await capabilities.trackGoal(userId, 'janitor', {
        type: 'test_coverage',
        title: 'Goal 2',
      });

      // Update only goal1
      await capabilities.updateGoalProgress(userId, goal1.id, 0.8);

      const g1 = await repos.goals.findById(goal1.id);
      const g2 = await repos.goals.findById(goal2.id);

      assert.equal(g1.progress, 0.8);
      assert.equal(g2.progress, 0);
    });
  });

  describe('Self-Correction Integration', () => {
    it('warns on dangerous actions', async () => {
      const userId = 'user1';

      const check = await capabilities.checkLessons(userId, 'danger: deleting all files');

      assert.equal(check.warning, true);
      assert.ok(check.message);
    });

    it('allows safe actions', async () => {
      const userId = 'user1';

      const check = await capabilities.checkLessons(userId, 'safe: reading a file');

      assert.equal(check.warning, false);
    });

    it('learns from mistakes', async () => {
      const userId = 'user1';

      const lesson = await capabilities.learnLesson(userId, {
        category: 'bug',
        mistake: 'Forgot to handle null response',
        correction: 'Added null check',
        prevention: 'Always validate API responses',
        severity: 'high',
      });

      assert.ok(lesson);
      assert.equal(lesson.category, 'bug');
      assert.equal(repos.memory._lessons.length, 1);
    });

    it('records architectural decisions', async () => {
      const userId = 'user1';

      const decision = await capabilities.recordDecision(userId, {
        type: 'pattern',
        title: 'Use Repository Pattern',
        description: 'Separate data access layer',
        rationale: 'Better testability and separation of concerns',
      });

      assert.ok(decision);
      // recordDecision maps type -> decisionType
      assert.equal(decision.decisionType, 'pattern');
      assert.equal(repos.memory._decisions.length, 1);
    });
  });

  describe('Notification Throttling', () => {
    it('respects importance threshold (φ⁻²)', async () => {
      const userId = 'user1';

      // Low importance - should be rejected
      const lowNotif = await capabilities.notify(userId, {
        title: 'Low Priority',
        message: 'Not important',
        importance: 0.1, // Below φ⁻² ≈ 0.382
      });

      assert.equal(lowNotif, null, 'Low importance notification should be rejected');

      // High importance - should be accepted
      const highNotif = await capabilities.notify(userId, {
        title: 'High Priority',
        message: 'Very important',
        importance: 0.5, // Above φ⁻²
      });

      assert.ok(highNotif, 'High importance notification should be accepted');
    });

    it('respects session notification limit', async () => {
      const userId = 'user1';
      const limit = AUTONOMOUS_CONSTANTS.MAX_NOTIFICATIONS_PER_SESSION;

      // Create max notifications
      for (let i = 0; i < limit; i++) {
        await capabilities.notify(userId, {
          title: `Notification ${i}`,
          message: 'Test',
          importance: 0.5,
        });
      }

      assert.equal(capabilities.getSessionNotificationCount(), limit);

      // Next should be rejected
      const excess = await capabilities.notify(userId, {
        title: 'Excess',
        message: 'Too many',
        importance: 0.9, // Even high importance
      });

      assert.equal(excess, null, 'Excess notification should be rejected');

      // Reset should allow new notifications
      capabilities.resetSessionNotifications();
      assert.equal(capabilities.getSessionNotificationCount(), 0);

      const afterReset = await capabilities.notify(userId, {
        title: 'After Reset',
        message: 'Should work',
        importance: 0.5,
      });

      assert.ok(afterReset, 'Notification after reset should work');
    });
  });
});

// ============================================================================
// INTEGRATION TEST: DOG GOAL TYPES
// ============================================================================

describe('Phase 16 Integration: Dog Goal Types', () => {
  it('all dogs have autonomous goals', () => {
    const dogs = Object.values(DogGoalTypes);
    assert.equal(dogs.length, 5, 'Should have 5 dogs');

    for (const dog of dogs) {
      assert.ok(dog.id, 'Dog should have id');
      assert.ok(dog.name, 'Dog should have name');
      assert.ok(Array.isArray(dog.goals), 'Dog should have goals array');
      assert.ok(dog.goals.length > 0, `${dog.name} should have at least one goal`);
    }
  });

  it('Guardian goals align with security', () => {
    const guardian = DogGoalTypes.GUARDIAN;
    const securityKeywords = ['security', 'vulnerability', 'threat', 'audit', 'scan'];

    const hasSecurityGoals = guardian.goals.some(g =>
      securityKeywords.some(kw => g.includes(kw))
    );

    assert.ok(hasSecurityGoals, 'Guardian should have security-related goals');
  });

  it('Janitor goals align with quality', () => {
    const janitor = DogGoalTypes.JANITOR;
    const qualityKeywords = ['test', 'coverage', 'quality', 'lint', 'clean'];

    const hasQualityGoals = janitor.goals.some(g =>
      qualityKeywords.some(kw => g.includes(kw))
    );

    assert.ok(hasQualityGoals, 'Janitor should have quality-related goals');
  });
});

// ============================================================================
// INTEGRATION TEST: φ-ALIGNED BEHAVIOR
// ============================================================================

describe('Phase 16 Integration: φ-Aligned Behavior', () => {
  it('canActAutonomously uses φ⁻¹ threshold', () => {
    const capabilities = createAutonomousCapabilities();

    // Below φ⁻¹ (0.618)
    assert.equal(capabilities.canActAutonomously(0.5), false);
    assert.equal(capabilities.canActAutonomously(0.6), false);

    // At φ⁻¹
    assert.equal(capabilities.canActAutonomously(PHI_INV), true);

    // Above φ⁻¹
    assert.equal(capabilities.canActAutonomously(0.7), true);
    assert.equal(capabilities.canActAutonomously(1.0), true);
  });

  it('notification importance uses φ⁻² threshold', () => {
    const threshold = AUTONOMOUS_CONSTANTS.MIN_NOTIFICATION_IMPORTANCE;

    // φ⁻² ≈ 0.382
    assert.ok(Math.abs(threshold - PHI_INV_2) < 0.001);
  });

  it('Fibonacci timing is correct', () => {
    const { FIBONACCI_TIMING } = AUTONOMOUS_CONSTANTS;

    assert.deepEqual(FIBONACCI_TIMING, [1, 2, 3, 5, 8, 13, 21, 34, 55]);

    // Verify each is sum of previous two (after first two)
    for (let i = 2; i < FIBONACCI_TIMING.length; i++) {
      assert.equal(
        FIBONACCI_TIMING[i],
        FIBONACCI_TIMING[i - 1] + FIBONACCI_TIMING[i - 2],
        `Fibonacci[${i}] should be sum of previous two`
      );
    }
  });
});

// ============================================================================
// INTEGRATION TEST: CONCURRENT OPERATIONS
// ============================================================================

describe('Phase 16 Integration: Concurrent Operations', () => {
  let capabilities;
  let repos;

  beforeEach(() => {
    repos = createMockRepositories();
    capabilities = createAutonomousCapabilities(repos);
  });

  it('handles concurrent goal updates', async () => {
    const userId = 'user1';

    const goal = await capabilities.trackGoal(userId, 'guardian', {
      type: 'security_audit',
      title: 'Concurrent Test',
    });

    // Concurrent progress updates
    const updates = [
      capabilities.updateGoalProgress(userId, goal.id, 0.2),
      capabilities.updateGoalProgress(userId, goal.id, 0.3),
      capabilities.updateGoalProgress(userId, goal.id, 0.1),
    ];

    await Promise.all(updates);

    const updatedGoal = await repos.goals.findById(goal.id);
    // Note: Exact progress depends on mock implementation
    // In real DB, should be sum capped at 1.0
    assert.ok(updatedGoal.progress >= 0.6);
  });

  it('handles concurrent task scheduling', async () => {
    const userId = 'user1';

    const taskPromises = [];
    for (let i = 0; i < 5; i++) {
      taskPromises.push(
        capabilities.scheduleTask(userId, `task_${i}`, { index: i })
      );
    }

    const tasks = await Promise.all(taskPromises);

    assert.equal(tasks.length, 5);
    assert.ok(tasks.every(t => t !== null));

    // All should have unique IDs
    const ids = new Set(tasks.map(t => t.id));
    assert.equal(ids.size, 5, 'All tasks should have unique IDs');
  });

  it('handles concurrent notifications within limit', async () => {
    const userId = 'user1';
    const limit = AUTONOMOUS_CONSTANTS.MAX_NOTIFICATIONS_PER_SESSION;

    // Try to create more than limit concurrently
    const promises = [];
    for (let i = 0; i < limit + 3; i++) {
      promises.push(
        capabilities.notify(userId, {
          title: `Notification ${i}`,
          message: 'Test',
          importance: 0.5,
        })
      );
    }

    const results = await Promise.all(promises);
    const created = results.filter(r => r !== null);

    // Should not exceed limit
    assert.ok(created.length <= limit, `Should not exceed limit of ${limit}`);
  });
});

// ============================================================================
// INTEGRATION TEST: ERROR HANDLING
// ============================================================================

describe('Phase 16 Integration: Error Handling', () => {
  it('handles missing repositories gracefully', async () => {
    const capabilities = createAutonomousCapabilities(); // No repos

    const goal = await capabilities.trackGoal('user1', 'guardian', { type: 'test' });
    assert.equal(goal, null, 'Should return null without repos');

    const task = await capabilities.scheduleTask('user1', 'test', {});
    assert.equal(task, null, 'Should return null without repos');

    const notification = await capabilities.notify('user1', { title: 'test' });
    assert.equal(notification, null, 'Should return null without repos');
  });

  it('checkLessons returns safe default without repos', async () => {
    const capabilities = createAutonomousCapabilities();

    const result = await capabilities.checkLessons('user1', 'any action');

    assert.deepEqual(result, { warning: false });
  });
});
