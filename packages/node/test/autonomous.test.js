/**
 * @cynic/node - Autonomous Capabilities Tests
 *
 * Tests for Phase 16: Total Memory + Full Autonomy
 * - Autonomous capabilities mixin
 * - Dog-specific behaviors
 * - CollectivePack integration
 *
 * "φ acts without being asked" - κυνικός
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  AUTONOMOUS_CONSTANTS,
  GOAL_TYPES,
  DogGoalTypes,
  getDogsForGoalType,
  getGoalTypesForDog,
  calculateGoalScore,
  getAllMetrics,
  createAutonomousCapabilities,
  DogAutonomousBehaviors,
} from '../src/agents/collective/autonomous.js';

import {
  CollectivePack,
  createCollectivePack,
} from '../src/agents/collective/index.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('AUTONOMOUS_CONSTANTS', () => {
  it('has Fibonacci timing array', () => {
    const { FIBONACCI_TIMING } = AUTONOMOUS_CONSTANTS;
    assert.ok(Array.isArray(FIBONACCI_TIMING));
    assert.equal(FIBONACCI_TIMING.length, 9);
    assert.deepEqual(FIBONACCI_TIMING, [1, 2, 3, 5, 8, 13, 21, 34, 55]);
  });

  it('has φ-aligned confidence thresholds', () => {
    assert.equal(AUTONOMOUS_CONSTANTS.MIN_CONFIDENCE, PHI_INV);
    assert.equal(AUTONOMOUS_CONSTANTS.MIN_NOTIFICATION_IMPORTANCE, PHI_INV_2);
  });

  it('has session limits', () => {
    assert.equal(AUTONOMOUS_CONSTANTS.MAX_NOTIFICATIONS_PER_SESSION, 5);
    assert.equal(AUTONOMOUS_CONSTANTS.PROGRESS_NOTIFY_THRESHOLD, 0.21);
  });
});

// =============================================================================
// DOG GOAL TYPES TESTS
// =============================================================================

describe('DogGoalTypes', () => {
  it('has all five dogs with autonomous behaviors', () => {
    const dogs = Object.keys(DogGoalTypes);
    assert.deepEqual(dogs.sort(), ['ARCHITECT', 'GUARDIAN', 'JANITOR', 'SAGE', 'SCHOLAR']);
  });

  it('Guardian (Shadow) has security goals', () => {
    const guardian = DogGoalTypes.GUARDIAN;
    assert.equal(guardian.id, 'guardian');
    assert.equal(guardian.name, 'Shadow');
    assert.ok(guardian.goals.includes('security_audit'));
    assert.ok(guardian.goals.includes('vulnerability_scan'));
    assert.ok(guardian.goals.includes('threat_monitoring'));
  });

  it('Architect (Max) has decision goals', () => {
    const architect = DogGoalTypes.ARCHITECT;
    assert.equal(architect.id, 'architect');
    assert.equal(architect.name, 'Max');
    assert.ok(architect.goals.includes('decision_tracking'));
    assert.ok(architect.goals.includes('drift_detection'));
  });

  it('Janitor (Ralph) has quality goals', () => {
    const janitor = DogGoalTypes.JANITOR;
    assert.equal(janitor.id, 'janitor');
    assert.equal(janitor.name, 'Ralph');
    assert.ok(janitor.goals.includes('test_coverage'));
    assert.ok(janitor.goals.includes('code_quality'));
  });

  it('Scholar (Archie) has memory goals', () => {
    const scholar = DogGoalTypes.SCHOLAR;
    assert.equal(scholar.id, 'scholar');
    assert.equal(scholar.name, 'Archie');
    assert.ok(scholar.goals.includes('memory_compaction'));
    assert.ok(scholar.goals.includes('knowledge_synthesis'));
  });

  it('Sage (Luna) has insight goals', () => {
    const sage = DogGoalTypes.SAGE;
    assert.equal(sage.id, 'sage');
    assert.equal(sage.name, 'Luna');
    assert.ok(sage.goals.includes('user_insights'));
    assert.ok(sage.goals.includes('proactive_guidance'));
  });
});

// =============================================================================
// CREATE AUTONOMOUS CAPABILITIES TESTS
// =============================================================================

describe('createAutonomousCapabilities', () => {
  it('creates capabilities without repositories', () => {
    const capabilities = createAutonomousCapabilities();
    assert.ok(capabilities);
    assert.equal(typeof capabilities.canActAutonomously, 'function');
    assert.equal(typeof capabilities.trackGoal, 'function');
    assert.equal(typeof capabilities.scheduleTask, 'function');
    assert.equal(typeof capabilities.notify, 'function');
    assert.equal(typeof capabilities.checkLessons, 'function');
    assert.equal(typeof capabilities.learnLesson, 'function');
    assert.equal(typeof capabilities.recordDecision, 'function');
    assert.equal(typeof capabilities.updateGoalProgress, 'function');
    assert.equal(typeof capabilities.resetSessionNotifications, 'function');
    assert.equal(typeof capabilities.getSessionNotificationCount, 'function');
  });

  it('canActAutonomously respects φ⁻¹ threshold', () => {
    const capabilities = createAutonomousCapabilities();

    // Below threshold
    assert.equal(capabilities.canActAutonomously(0.5), false);
    assert.equal(capabilities.canActAutonomously(0.6), false);

    // At or above threshold (φ⁻¹ ≈ 0.618)
    assert.equal(capabilities.canActAutonomously(PHI_INV), true);
    assert.equal(capabilities.canActAutonomously(0.7), true);
    assert.equal(capabilities.canActAutonomously(1.0), true);
  });

  it('returns null for operations when no repositories', async () => {
    const capabilities = createAutonomousCapabilities();

    const goal = await capabilities.trackGoal('user1', 'guardian', { type: 'test' });
    assert.equal(goal, null);

    const task = await capabilities.scheduleTask('user1', 'test', {});
    assert.equal(task, null);

    const notification = await capabilities.notify('user1', { title: 'test' });
    assert.equal(notification, null);

    const check = await capabilities.checkLessons('user1', 'test');
    assert.deepEqual(check, { warning: false });

    const lesson = await capabilities.learnLesson('user1', { mistake: 'test' });
    assert.equal(lesson, null);

    const decision = await capabilities.recordDecision('user1', { title: 'test' });
    assert.equal(decision, null);

    const progress = await capabilities.updateGoalProgress('user1', 'goal1', 0.1);
    assert.equal(progress, null);
  });

  it('tracks session notification count', async () => {
    const capabilities = createAutonomousCapabilities();

    assert.equal(capabilities.getSessionNotificationCount(), 0);

    // Even without repos, notify should track attempts
    await capabilities.notify('user1', { title: 'test', importance: 0.5 });
    // Count doesn't increment without repos (returns early)
    assert.equal(capabilities.getSessionNotificationCount(), 0);

    capabilities.resetSessionNotifications();
    assert.equal(capabilities.getSessionNotificationCount(), 0);
  });

  describe('with mock repositories', () => {
    let capabilities;
    let mockGoals;
    let mockTasks;
    let mockNotifications;
    let mockMemory;

    beforeEach(() => {
      mockGoals = {
        findByUser: async () => [],
        create: async (data) => ({ id: 'goal-1', ...data }),
        findById: async (id) => ({ id, title: 'Test Goal', progress: 0.5, status: 'active' }),
        updateProgress: async (id, progress) => ({ id, progress }),
        complete: async (id) => ({ id, status: 'completed' }),
      };

      mockTasks = {
        create: async (data) => ({ id: 'task-1', ...data }),
      };

      mockNotifications = {
        create: async (data) => ({ id: 'notif-1', ...data }),
      };

      mockMemory = {
        checkForMistakes: async (userId, action) => ({
          warning: action.includes('danger'),
          message: action.includes('danger') ? 'Previous mistake found' : null,
          lessons: [],
        }),
        rememberLesson: async (userId, lesson) => ({ id: 'lesson-1', ...lesson }),
        rememberDecision: async (userId, decision) => ({ id: 'decision-1', ...decision }),
      };

      capabilities = createAutonomousCapabilities({
        goals: mockGoals,
        tasks: mockTasks,
        notifications: mockNotifications,
        memory: mockMemory,
      });
    });

    it('tracks goals with repositories', async () => {
      const goal = await capabilities.trackGoal('user1', 'guardian', {
        type: 'security_audit',
        title: 'Security Check',
        description: 'Audit the codebase',
      });

      assert.ok(goal);
      assert.equal(goal.id, 'goal-1');
      assert.equal(goal.goalType, 'security_audit');
    });

    it('schedules tasks with repositories', async () => {
      const task = await capabilities.scheduleTask('user1', 'analyze', {
        data: 'test',
      });

      assert.ok(task);
      assert.equal(task.id, 'task-1');
      assert.equal(task.taskType, 'analyze');
    });

    it('creates notifications with sufficient importance', async () => {
      const notification = await capabilities.notify('user1', {
        type: 'insight',
        title: 'Test Insight',
        message: 'This is a test',
        importance: 0.5, // Above φ⁻² threshold
      });

      assert.ok(notification);
      assert.equal(notification.id, 'notif-1');
      assert.equal(capabilities.getSessionNotificationCount(), 1);
    });

    it('rejects notifications with low importance', async () => {
      const notification = await capabilities.notify('user1', {
        title: 'Low Priority',
        importance: 0.1, // Below φ⁻² threshold
      });

      assert.equal(notification, null);
      assert.equal(capabilities.getSessionNotificationCount(), 0);
    });

    it('respects session notification limit', async () => {
      // Create 5 notifications (max)
      for (let i = 0; i < 5; i++) {
        await capabilities.notify('user1', {
          title: `Notification ${i}`,
          importance: 0.5,
        });
      }

      assert.equal(capabilities.getSessionNotificationCount(), 5);

      // 6th should be rejected
      const excess = await capabilities.notify('user1', {
        title: 'Excess Notification',
        importance: 0.9,
      });

      assert.equal(excess, null);
      assert.equal(capabilities.getSessionNotificationCount(), 5);
    });

    it('checks lessons for warnings', async () => {
      const noWarning = await capabilities.checkLessons('user1', 'safe action');
      assert.equal(noWarning.warning, false);

      const warning = await capabilities.checkLessons('user1', 'danger action');
      assert.equal(warning.warning, true);
      assert.ok(warning.message);
    });

    it('learns lessons', async () => {
      const lesson = await capabilities.learnLesson('user1', {
        category: 'bug',
        mistake: 'Forgot null check',
        correction: 'Add null check',
        prevention: 'Always validate input',
        severity: 'medium',
      });

      assert.ok(lesson);
      assert.equal(lesson.id, 'lesson-1');
    });

    it('records decisions', async () => {
      const decision = await capabilities.recordDecision('user1', {
        type: 'pattern',
        title: 'Use Repository Pattern',
        description: 'Separate data access',
        rationale: 'Better testability',
      });

      assert.ok(decision);
      assert.equal(decision.id, 'decision-1');
    });

    it('updates goal progress', async () => {
      const updated = await capabilities.updateGoalProgress('user1', 'goal-1', 0.2);

      assert.ok(updated);
      assert.equal(updated.progress, 0.7); // 0.5 + 0.2
    });
  });
});

// =============================================================================
// DOG AUTONOMOUS BEHAVIORS TESTS
// =============================================================================

describe('DogAutonomousBehaviors', () => {
  it('has behaviors for all five dogs', () => {
    const dogs = Object.keys(DogAutonomousBehaviors);
    assert.deepEqual(dogs.sort(), ['architect', 'guardian', 'janitor', 'sage', 'scholar']);
  });

  describe('guardian behaviors', () => {
    it('has monitorSecurity and reportThreat', () => {
      assert.equal(typeof DogAutonomousBehaviors.guardian.monitorSecurity, 'function');
      assert.equal(typeof DogAutonomousBehaviors.guardian.reportThreat, 'function');
    });
  });

  describe('architect behaviors', () => {
    it('has trackDecision and detectDrift', () => {
      assert.equal(typeof DogAutonomousBehaviors.architect.trackDecision, 'function');
      assert.equal(typeof DogAutonomousBehaviors.architect.detectDrift, 'function');
    });
  });

  describe('janitor behaviors', () => {
    it('has monitorCoverage and reportQuality', () => {
      assert.equal(typeof DogAutonomousBehaviors.janitor.monitorCoverage, 'function');
      assert.equal(typeof DogAutonomousBehaviors.janitor.reportQuality, 'function');
    });
  });

  describe('scholar behaviors', () => {
    it('has compactMemory and synthesizeKnowledge', () => {
      assert.equal(typeof DogAutonomousBehaviors.scholar.compactMemory, 'function');
      assert.equal(typeof DogAutonomousBehaviors.scholar.synthesizeKnowledge, 'function');
    });
  });

  describe('sage behaviors', () => {
    it('has generateInsight and trackLearning', () => {
      assert.equal(typeof DogAutonomousBehaviors.sage.generateInsight, 'function');
      assert.equal(typeof DogAutonomousBehaviors.sage.trackLearning, 'function');
    });
  });
});

// =============================================================================
// COLLECTIVE PACK INTEGRATION TESTS
// =============================================================================

// TODO: Fix cleanup - CollectivePack keeps Node running
describe.skip('CollectivePack with autonomous capabilities', () => {
  it('creates pack without autonomous repositories', () => {
    const pack = createCollectivePack();
    assert.ok(pack);
    assert.equal(pack.hasAutonomousCapabilities(), false);
    assert.equal(pack.getAutonomousCapabilities(), null);
  });

  it('creates pack with autonomous repositories', () => {
    const mockRepos = {
      goals: { create: async () => ({}) },
      tasks: { create: async () => ({}) },
      notifications: { create: async () => ({}) },
      memory: { checkForMistakes: async () => ({ warning: false }) },
    };

    const pack = createCollectivePack({ autonomousRepositories: mockRepos });
    assert.ok(pack);
    assert.equal(pack.hasAutonomousCapabilities(), true);
    assert.ok(pack.getAutonomousCapabilities());
  });

  it('passes autonomous capabilities to dogs', () => {
    const mockRepos = {
      goals: { create: async () => ({}) },
      tasks: { create: async () => ({}) },
      notifications: { create: async () => ({}) },
      memory: { checkForMistakes: async () => ({ warning: false }) },
    };

    const pack = createCollectivePack({ autonomousRepositories: mockRepos });

    // Check that dogs received autonomous capabilities
    assert.ok(pack.guardian.autonomous);
    assert.ok(pack.architect.autonomous);
    assert.ok(pack.janitor.autonomous);
    assert.ok(pack.scholar.autonomous);
    assert.ok(pack.sage.autonomous);
  });
});

// =============================================================================
// GOAL_TYPES TESTS (Phase 4: Collective Enhancement)
// =============================================================================

describe('GOAL_TYPES', () => {
  it('has five goal type categories', () => {
    const types = Object.keys(GOAL_TYPES);
    assert.deepEqual(types.sort(), ['LEARNING', 'MAINTENANCE', 'MONITORING', 'QUALITY', 'SECURITY']);
  });

  it('QUALITY has correct structure', () => {
    const quality = GOAL_TYPES.QUALITY;
    assert.equal(quality.name, 'Code Quality');
    assert.ok(quality.description);
    assert.ok(Array.isArray(quality.metrics));
    assert.ok(quality.metrics.includes('test_coverage'));
    assert.ok(quality.metrics.includes('lint_score'));
    assert.deepEqual(quality.dogs, ['Ralph', 'Max']);
    assert.equal(quality.targetWeight, PHI_INV);
  });

  it('LEARNING has correct structure', () => {
    const learning = GOAL_TYPES.LEARNING;
    assert.equal(learning.name, 'Learning');
    assert.ok(learning.metrics.includes('lessons_applied'));
    assert.ok(learning.metrics.includes('mistakes_prevented'));
    assert.deepEqual(learning.dogs, ['Archie', 'Luna']);
    assert.equal(learning.targetWeight, PHI_INV_2);
  });

  it('SECURITY has correct structure', () => {
    const security = GOAL_TYPES.SECURITY;
    assert.equal(security.name, 'Security');
    assert.ok(security.metrics.includes('vulnerabilities_fixed'));
    assert.deepEqual(security.dogs, ['Shadow']);
    assert.equal(security.targetWeight, PHI_INV);
  });

  it('MAINTENANCE has correct structure', () => {
    const maintenance = GOAL_TYPES.MAINTENANCE;
    assert.equal(maintenance.name, 'Maintenance');
    assert.ok(maintenance.metrics.includes('tech_debt_reduced'));
    assert.deepEqual(maintenance.dogs, ['Max', 'Ralph']);
  });

  it('MONITORING has correct structure', () => {
    const monitoring = GOAL_TYPES.MONITORING;
    assert.equal(monitoring.name, 'Monitoring');
    assert.ok(monitoring.metrics.includes('health_checks'));
    assert.deepEqual(monitoring.dogs, ['Shadow', 'Ralph']);
  });
});

// =============================================================================
// GOAL_TYPES HELPER FUNCTIONS TESTS
// =============================================================================

describe('getDogsForGoalType', () => {
  it('returns dogs for QUALITY', () => {
    const dogs = getDogsForGoalType('QUALITY');
    assert.deepEqual(dogs, ['Ralph', 'Max']);
  });

  it('returns dogs for SECURITY', () => {
    const dogs = getDogsForGoalType('security');
    assert.deepEqual(dogs, ['Shadow']);
  });

  it('returns empty array for unknown type', () => {
    const dogs = getDogsForGoalType('UNKNOWN');
    assert.deepEqual(dogs, []);
  });
});

describe('getGoalTypesForDog', () => {
  it('returns goal types for Ralph', () => {
    const types = getGoalTypesForDog('Ralph');
    assert.ok(types.includes('QUALITY'));
    assert.ok(types.includes('MAINTENANCE'));
    assert.ok(types.includes('MONITORING'));
  });

  it('returns goal types for Shadow', () => {
    const types = getGoalTypesForDog('Shadow');
    assert.ok(types.includes('SECURITY'));
    assert.ok(types.includes('MONITORING'));
  });

  it('returns goal types for Luna', () => {
    const types = getGoalTypesForDog('Luna');
    assert.ok(types.includes('LEARNING'));
  });

  it('is case insensitive', () => {
    const types = getGoalTypesForDog('RALPH');
    assert.ok(types.includes('QUALITY'));
  });

  it('returns empty array for unknown dog', () => {
    const types = getGoalTypesForDog('Unknown');
    assert.deepEqual(types, []);
  });
});

describe('calculateGoalScore', () => {
  it('calculates weighted score for QUALITY', () => {
    const score = calculateGoalScore('QUALITY', {
      test_coverage: 0.8,
      lint_score: 0.9,
      complexity: 0.7,
    });

    // Average: (0.8 + 0.9 + 0.7) / 3 = 0.8
    // Weighted: 0.8 * PHI_INV ≈ 0.494
    assert.ok(score > 0.4 && score < 0.6);
  });

  it('handles partial metrics', () => {
    const score = calculateGoalScore('QUALITY', {
      test_coverage: 0.8,
    });

    // 0.8 * PHI_INV ≈ 0.494
    assert.ok(score > 0.4 && score < 0.6);
  });

  it('returns 0 for unknown goal type', () => {
    const score = calculateGoalScore('UNKNOWN', { metric: 0.8 });
    assert.equal(score, 0);
  });

  it('returns 0 for empty metrics', () => {
    const score = calculateGoalScore('QUALITY', {});
    assert.equal(score, 0);
  });
});

describe('getAllMetrics', () => {
  it('returns all unique metrics', () => {
    const metrics = getAllMetrics();

    assert.ok(Array.isArray(metrics));
    assert.ok(metrics.includes('test_coverage'));
    assert.ok(metrics.includes('lint_score'));
    assert.ok(metrics.includes('vulnerabilities_fixed'));
    assert.ok(metrics.includes('lessons_applied'));
    assert.ok(metrics.includes('tech_debt_reduced'));
    assert.ok(metrics.includes('health_checks'));
  });

  it('has no duplicates', () => {
    const metrics = getAllMetrics();
    const unique = new Set(metrics);
    assert.equal(metrics.length, unique.size);
  });
});
