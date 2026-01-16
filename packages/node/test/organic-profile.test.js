/**
 * CYNIC Organic Profile Tests
 *
 * Tests for 100% automatic profile detection from user behavior.
 * No explicit declaration - profile emerges from interaction patterns.
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  SIGNAL_CONSTANTS,
  SignalType,
  // Linguistic
  analyzeVocabulary,
  classifyQuestion,
  calculateTechnicalDensity,
  detectSelfCorrection,
  calculateLinguisticSignal,
  // Behavioral
  TOOL_COMPLEXITY,
  calculateToolComplexity,
  calculateErrorRecovery,
  calculateIterationDepth,
  calculateBehavioralSignal,
  // Code
  analyzeAbstractionLevel,
  scoreErrorHandling,
  analyzeTestingAwareness,
  countArchitecturalPatterns,
  calculateCodeSignal,
  // Temporal
  calculateLearningRate,
  calculateConsistency,
  calculateEngagementDepth,
  calculateTemporalSignal,
  // Collector
  OrganicSignals,
} from '../src/profile/organic-signals.js';

import {
  PROFILE_CONSTANTS,
  ProfileLevel,
  ProfileState,
  ProfileCalculator,
  createProfileCalculator,
} from '../src/profile/calculator.js';

import { PHI_INV } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// LINGUISTIC SIGNAL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Linguistic Signals', () => {
  describe('analyzeVocabulary', () => {
    it('scores technical messages higher', () => {
      const technical = 'We need to implement async/await with proper error handling using try-catch and GraphQL mutations';
      const basic = 'Can you help me fix this code please?';

      const techScore = analyzeVocabulary(technical);
      const basicScore = analyzeVocabulary(basic);

      assert.ok(techScore > basicScore);
    });

    it('returns 50 for empty input', () => {
      assert.strictEqual(analyzeVocabulary(''), 50);
      assert.strictEqual(analyzeVocabulary(null), 50);
    });

    it('considers word diversity', () => {
      const diverse = 'The architecture includes services, controllers, repositories, and middleware components';
      const repetitive = 'The code code code needs code changes code';

      const diverseScore = analyzeVocabulary(diverse);
      const repScore = analyzeVocabulary(repetitive);

      assert.ok(diverseScore > repScore);
    });
  });

  describe('classifyQuestion', () => {
    it('identifies "what" questions as basic', () => {
      const { depth, type } = classifyQuestion('What is a function?');
      assert.strictEqual(type, 'what');
      assert.strictEqual(depth, 25);
    });

    it('identifies "why" questions as intermediate', () => {
      const { depth, type } = classifyQuestion('Why does JavaScript have closures?');
      assert.strictEqual(type, 'why');
      assert.strictEqual(depth, 50);
    });

    it('identifies "how" questions as applied', () => {
      const { depth, type } = classifyQuestion('How do I implement authentication?');
      assert.strictEqual(type, 'how');
      assert.strictEqual(depth, 65);
    });

    it('identifies trade-off questions as expert', () => {
      const { depth, type } = classifyQuestion('What are the trade-offs between SQL and NoSQL?');
      assert.strictEqual(type, 'tradeoff');
      assert.strictEqual(depth, 85);
    });

    it('identifies architecture questions as master', () => {
      const { depth, type } = classifyQuestion('How should I design a microservices architecture?');
      assert.strictEqual(type, 'architecture');
      assert.strictEqual(depth, 95);
    });
  });

  describe('calculateTechnicalDensity', () => {
    it('calculates density correctly', () => {
      const technical = 'async function api database query index';
      const basic = 'please help me fix this issue';

      const techDensity = calculateTechnicalDensity(technical);
      const basicDensity = calculateTechnicalDensity(basic);

      assert.ok(techDensity > basicDensity);
      assert.ok(techDensity > 0.5);
      assert.ok(basicDensity < 0.3);
    });
  });

  describe('detectSelfCorrection', () => {
    it('detects correction patterns', () => {
      const history = [
        'I think we should use SQL',
        'Actually wait, let me rephrase that',
        'We should use NoSQL instead',
      ];

      const rate = detectSelfCorrection(history);
      assert.ok(rate > 0);
    });

    it('returns 0 for no corrections', () => {
      const history = [
        'Use this approach',
        'Then add this',
        'Finally do this',
      ];

      const rate = detectSelfCorrection(history);
      assert.strictEqual(rate, 0);
    });
  });

  describe('calculateLinguisticSignal', () => {
    it('combines all linguistic factors', () => {
      const expertMessage = 'What are the trade-offs between implementing a distributed cache with Redis versus Memcached for high-availability microservices?';

      const result = calculateLinguisticSignal(expertMessage);

      assert.ok(result.score > 60);
      assert.ok(result.breakdown.vocabulary > 50);
      assert.ok(result.breakdown.questionDepth >= 85);
    });

    it('scores novice messages lower', () => {
      const noviceMessage = 'What is a variable?';

      const result = calculateLinguisticSignal(noviceMessage);

      assert.ok(result.score < 40);
      assert.strictEqual(result.breakdown.questionDepth, 25);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// BEHAVIORAL SIGNAL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Behavioral Signals', () => {
  describe('calculateToolComplexity', () => {
    it('scores complex tool usage higher', () => {
      const expertUsage = [
        { tool: 'architect', success: true },
        { tool: 'refactor', success: true },
        { tool: 'review', success: true },
      ];

      const noviceUsage = [
        { tool: 'read_file', success: true },
        { tool: 'list_dir', success: true },
        { tool: 'search', success: true },
      ];

      const expertResult = calculateToolComplexity(expertUsage);
      const noviceResult = calculateToolComplexity(noviceUsage);

      assert.ok(expertResult.score > noviceResult.score);
      assert.ok(expertResult.avgComplexity > 4);
      assert.ok(noviceResult.avgComplexity < 2);
    });

    it('considers success rate', () => {
      const successful = [
        { tool: 'edit', success: true },
        { tool: 'edit', success: true },
      ];

      const failing = [
        { tool: 'edit', success: false },
        { tool: 'edit', success: false },
      ];

      const successResult = calculateToolComplexity(successful);
      const failResult = calculateToolComplexity(failing);

      assert.ok(successResult.score > failResult.score);
    });
  });

  describe('calculateErrorRecovery', () => {
    it('scores fast recovery higher', () => {
      const now = Date.now();

      const fastRecovery = [
        { timestamp: now, isError: true },
        { timestamp: now + 5000, isError: false }, // 5s recovery
      ];

      const slowRecovery = [
        { timestamp: now, isError: true },
        { timestamp: now + 300000, isError: false }, // 5 min recovery
      ];

      const fastResult = calculateErrorRecovery(fastRecovery);
      const slowResult = calculateErrorRecovery(slowRecovery);

      assert.ok(fastResult.score > slowResult.score);
    });

    it('scores no errors as good', () => {
      const noErrors = [
        { timestamp: Date.now(), isError: false },
        { timestamp: Date.now() + 1000, isError: false },
      ];

      const result = calculateErrorRecovery(noErrors);
      assert.strictEqual(result.score, 75);
    });
  });

  describe('calculateIterationDepth', () => {
    it('scores fewer iterations higher when satisfied', () => {
      const oneIteration = calculateIterationDepth(1, true);
      const manyIterations = calculateIterationDepth(10, true);

      assert.ok(oneIteration > manyIterations);
      assert.ok(oneIteration >= 95);
    });

    it('scores unsatisfied lower', () => {
      const unsatisfied = calculateIterationDepth(3, false);
      const satisfied = calculateIterationDepth(3, true);

      assert.ok(satisfied > unsatisfied);
      assert.strictEqual(unsatisfied, 30);
    });
  });

  describe('calculateBehavioralSignal', () => {
    it('combines all behavioral factors', () => {
      const now = Date.now();

      const toolUsage = [
        { tool: 'architect', success: true },
        { tool: 'review', success: true },
      ];

      const events = [
        { timestamp: now, isError: true },
        { timestamp: now + 5000, isError: false },
      ];

      const result = calculateBehavioralSignal(toolUsage, events, {
        iterations: 2,
        satisfied: true,
      });

      assert.ok(result.score > 50);
      assert.ok(result.breakdown.toolComplexity > 0);
      assert.ok(result.breakdown.errorRecovery > 0);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// CODE SIGNAL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Code Signals', () => {
  describe('analyzeAbstractionLevel', () => {
    it('identifies basic code', () => {
      const basic = `
        var x = 1;
        console.log(x);
        if (x > 0) {
          console.log('positive');
        }
      `;

      const result = analyzeAbstractionLevel(basic);
      assert.strictEqual(result.level, 'basic');
      assert.ok(result.score < 40);
    });

    it('identifies OOP code', () => {
      const oop = `
        class UserService extends BaseService {
          private repository;

          constructor(repo) {
            super();
            this.repository = repo;
          }
        }
      `;

      const result = analyzeAbstractionLevel(oop);
      assert.strictEqual(result.level, 'oop');
      assert.ok(result.score >= 60);
    });

    it('identifies generics/types', () => {
      const generics = `
        interface Repository<T> {
          findById(id: string): Promise<T>;
        }

        type UserRepository = Repository<User>;
      `;

      const result = analyzeAbstractionLevel(generics);
      assert.strictEqual(result.level, 'generics');
      assert.ok(result.score >= 75);
    });
  });

  describe('scoreErrorHandling', () => {
    it('scores comprehensive error handling high', () => {
      const comprehensive = `
        try {
          const result = await apiCall();
        } catch (error: NetworkError) {
          console.error('Network error:', error);
          throw new CustomError('API failed');
        } finally {
          cleanup();
        }
      `;

      const result = scoreErrorHandling(comprehensive);
      assert.ok(result.score >= 70);
      assert.ok(result.patterns.includes('try-catch'));
      assert.ok(result.patterns.includes('error-logging'));
      assert.ok(result.patterns.includes('finally'));
    });

    it('scores no error handling low', () => {
      const none = `
        const x = getData();
        processData(x);
      `;

      const result = scoreErrorHandling(none);
      assert.ok(result.score < 50);
      assert.strictEqual(result.patterns.length, 0);
    });
  });

  describe('analyzeTestingAwareness', () => {
    it('detects Jest tests', () => {
      const jest = `
        describe('UserService', () => {
          beforeEach(() => {
            jest.clearAllMocks();
          });

          it('should create user', () => {
            expect(service.create()).toBeDefined();
          });
        });
      `;

      const result = analyzeTestingAwareness(jest);
      assert.ok(result.hasTests);
      assert.strictEqual(result.framework, 'jest');
      assert.ok(result.score >= 80);
    });

    it('detects Node.js test runner', () => {
      const nodeTest = `
        import { describe, it } from 'node:test';

        describe('feature', () => {
          it('works', () => {});
        });
      `;

      const result = analyzeTestingAwareness(nodeTest);
      assert.ok(result.hasTests);
      assert.strictEqual(result.framework, 'nodeTest');
    });
  });

  describe('countArchitecturalPatterns', () => {
    it('detects common patterns', () => {
      const code = `
        class UserRepository {
          findById(id) {}
          save(user) {}
        }

        class UserService {
          constructor(repository) {
            this.repository = repository;
          }
        }

        function createUser(data) {
          return new User(data);
        }
      `;

      const result = countArchitecturalPatterns(code);
      assert.ok(result.patterns.includes('repository'));
      assert.ok(result.patterns.includes('service'));
      assert.ok(result.patterns.includes('factory'));
      assert.ok(result.score >= 70);
    });
  });

  describe('calculateCodeSignal', () => {
    it('combines all code factors', () => {
      const expertCode = `
        interface IUserRepository<T extends User> {
          findById(id: string): Promise<T | null>;
        }

        @Injectable()
        class UserService {
          constructor(private repo: IUserRepository<User>) {}

          async getUser(id: string): Promise<Result<User, NotFoundError>> {
            try {
              const user = await this.repo.findById(id);
              return user ? Result.ok(user) : Result.err(new NotFoundError());
            } catch (error) {
              console.error('Failed to get user:', error);
              throw error;
            }
          }
        }

        describe('UserService', () => {
          it('returns user', async () => {
            const mock = jest.fn();
            expect(await service.getUser('1')).toBeDefined();
          });
        });
      `;

      const result = calculateCodeSignal(expertCode);
      assert.ok(result.score >= 70);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// TEMPORAL SIGNAL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Temporal Signals', () => {
  describe('calculateLearningRate', () => {
    it('detects improving trend', () => {
      const improving = [
        { value: 50, timestamp: 0 },
        { value: 55, timestamp: 1 },
        { value: 60, timestamp: 2 },
        { value: 65, timestamp: 3 },
        { value: 70, timestamp: 4 },
      ];

      const result = calculateLearningRate(improving);
      assert.ok(result.slope > 0);
      assert.ok(result.score > 50);
    });

    it('detects declining trend', () => {
      const declining = [
        { value: 70, timestamp: 0 },
        { value: 65, timestamp: 1 },
        { value: 60, timestamp: 2 },
        { value: 55, timestamp: 3 },
        { value: 50, timestamp: 4 },
      ];

      const result = calculateLearningRate(declining);
      assert.ok(result.slope < 0);
      assert.ok(result.score < 50);
    });
  });

  describe('calculateConsistency', () => {
    it('scores consistent signals high', () => {
      const consistent = [75, 76, 74, 75, 75, 74, 76];
      const result = calculateConsistency(consistent);

      assert.ok(result.score > 80);
      assert.ok(result.variance < 2);
    });

    it('scores inconsistent signals low', () => {
      const inconsistent = [30, 80, 45, 90, 20, 85];
      const result = calculateConsistency(inconsistent);

      assert.ok(result.score < 50);
      assert.ok(result.variance > 500);
    });
  });

  describe('calculateEngagementDepth', () => {
    it('scores long sessions higher', () => {
      const longSessions = [
        { durationMs: 3600000 }, // 1 hour
        { durationMs: 2700000 }, // 45 min
      ];

      const shortSessions = [
        { durationMs: 180000 }, // 3 min
        { durationMs: 120000 }, // 2 min
      ];

      const longResult = calculateEngagementDepth(longSessions);
      const shortResult = calculateEngagementDepth(shortSessions);

      assert.ok(longResult.score > shortResult.score);
      assert.ok(longResult.avgMinutes > 40);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// ORGANIC SIGNALS COLLECTOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('OrganicSignals', () => {
  let signals;

  beforeEach(() => {
    signals = new OrganicSignals();
  });

  it('starts with no signals', () => {
    const combined = signals.getCombinedScore();
    assert.strictEqual(combined.confidence, 0);
    assert.strictEqual(combined.level, 3); // Default
  });

  it('updates linguistic signal', () => {
    signals.updateLinguistic('What are the trade-offs in microservices architecture?');

    assert.ok(signals.linguistic);
    assert.ok(signals.linguistic.score > 60);
    assert.strictEqual(signals.sampleCounts.linguistic, 1);
  });

  it('combines signals with Fibonacci weights', () => {
    // Expert-level signals
    signals.updateLinguistic('How should I architect a distributed system with high availability?');
    signals.updateBehavioral([
      { tool: 'architect', success: true },
      { tool: 'review', success: true },
    ]);

    const combined = signals.getCombinedScore();
    assert.ok(combined.score > 60);
    assert.ok(combined.level >= 5); // Expert or higher
    assert.ok(combined.confidence > 0);
  });

  it('respects max confidence φ⁻¹', () => {
    // Add many samples
    for (let i = 0; i < 50; i++) {
      signals.updateLinguistic('technical message ' + i);
    }

    const combined = signals.getCombinedScore();
    assert.ok(combined.confidence <= PHI_INV);
  });

  it('provides full breakdown', () => {
    signals.updateLinguistic('test message');
    signals.updateBehavioral([{ tool: 'read_file', success: true }]);

    const breakdown = signals.getBreakdown();

    assert.ok(breakdown.linguistic);
    assert.ok(breakdown.behavioral);
    assert.ok(breakdown.combined);
    assert.ok(breakdown.sampleCounts);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// PROFILE CALCULATOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('ProfileCalculator', () => {
  let calculator;

  beforeEach(() => {
    calculator = createProfileCalculator();
  });

  describe('initial state', () => {
    it('starts at Practitioner level', () => {
      assert.strictEqual(calculator.getLevel(), ProfileLevel.PRACTITIONER);
      assert.strictEqual(calculator.getLevelName(), 'Practitioner');
    });

    it('has zero confidence initially', () => {
      const state = calculator.getState();
      assert.strictEqual(state.confidence, 0);
    });
  });

  describe('message processing', () => {
    it('processes messages and updates signals', () => {
      calculator.processMessage('How do I implement OAuth2 authentication?');

      const breakdown = calculator.getBreakdown();
      assert.ok(breakdown.signals.linguistic);
      assert.strictEqual(breakdown.stats.messageCount, 1);
    });

    it('maintains message history', () => {
      calculator.processMessage('First message');
      calculator.processMessage('Second message');
      calculator.processMessage('Third message');

      const breakdown = calculator.getBreakdown();
      assert.strictEqual(breakdown.stats.messageCount, 3);
    });
  });

  describe('tool processing', () => {
    it('processes tool calls', () => {
      calculator.processToolCall('read_file', true, false);
      calculator.processToolCall('edit', true, false);

      const breakdown = calculator.getBreakdown();
      assert.ok(breakdown.signals.behavioral);
      assert.strictEqual(breakdown.stats.toolCallCount, 2);
    });

    it('tracks error events', () => {
      calculator.processToolCall('bash', false, true);
      calculator.processToolCall('bash', true, false);

      const breakdown = calculator.getBreakdown();
      assert.ok(breakdown.signals.behavioral);
    });
  });

  describe('code processing', () => {
    it('processes code snippets', () => {
      calculator.processCode(`
        class Service {
          async getData() {
            try {
              return await this.fetch();
            } catch (e) {
              console.error(e);
            }
          }
        }
      `);

      const breakdown = calculator.getBreakdown();
      assert.ok(breakdown.signals.code);
      assert.strictEqual(breakdown.stats.codeSnippetCount, 1);
    });
  });

  describe('recalculation', () => {
    it('recalculates after threshold interactions', () => {
      // Process enough interactions to trigger recalculation
      for (let i = 0; i < PROFILE_CONSTANTS.REEVALUATION_INTERVAL; i++) {
        calculator.processMessage('How do I architect microservices? ' + i);
      }

      const state = calculator.getState();
      assert.ok(state.interactionCount >= PROFILE_CONSTANTS.REEVALUATION_INTERVAL);
    });

    it('can force recalculation', () => {
      calculator.processMessage('How should I design a distributed system?');
      calculator.processToolCall('architect', true);

      const result = calculator.recalculate(true);
      assert.ok(result.level > 0);
    });
  });

  describe('level adaptation', () => {
    it('provides adaptation hints', () => {
      const hints = calculator.getAdaptationHints();

      assert.ok(hints.explanationDepth);
      assert.ok(hints.terminology);
      assert.ok(hints.examples);
      assert.ok(hints.warnings);
      assert.ok(hints.assumeKnowledge);
    });

    it('adapts hints based on level', () => {
      // Simulate expert signals
      for (let i = 0; i < 30; i++) {
        calculator.processMessage('What are the trade-offs in implementing CQRS with event sourcing for microservices?');
        calculator.processToolCall('architect', true);
        calculator.processCode(`
          interface EventStore<T extends Event> {
            append(stream: string, events: T[]): Promise<void>;
          }
        `);
      }
      calculator.recalculate(true);

      const expertHints = calculator.getAdaptationHints();

      // Reset and simulate novice
      calculator.reset();
      for (let i = 0; i < 30; i++) {
        calculator.processMessage('What is a variable?');
        calculator.processToolCall('read_file', true);
        calculator.processCode('var x = 1; console.log(x);');
      }
      calculator.recalculate(true);

      const noviceHints = calculator.getAdaptationHints();

      // Hints should differ
      assert.notStrictEqual(expertHints.explanationDepth, noviceHints.explanationDepth);
    });
  });

  describe('persistence', () => {
    it('exports and imports backup', () => {
      calculator.processMessage('Test message');
      calculator.processToolCall('edit', true);

      const backup = calculator.exportForBackup();
      assert.ok(backup.state);
      assert.ok(backup.signalStore);
      assert.ok(backup.exportedAt);

      // Import into new calculator
      const newCalculator = createProfileCalculator();
      newCalculator.importFromBackup(backup);

      assert.strictEqual(
        newCalculator.getState().interactionCount,
        calculator.getState().interactionCount
      );
    });
  });

  describe('reset', () => {
    it('resets to initial state', () => {
      calculator.processMessage('Test');
      calculator.processToolCall('edit', true);

      calculator.reset();

      assert.strictEqual(calculator.getLevel(), ProfileLevel.PRACTITIONER);
      assert.strictEqual(calculator.getState().interactionCount, 0);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// PROFILE DETECTION SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════

describe('Profile Detection Scenarios', () => {
  it('detects novice profile organically', () => {
    const calculator = createProfileCalculator();

    // Simulate novice behavior
    const noviceMessages = [
      'What is JavaScript?',
      'What is a variable?',
      'How do I print hello world?',
      'What is a function?',
      'Help me fix this code',
    ];

    const noviceCode = `
      var x = 1;
      var y = 2;
      console.log(x + y);
    `;

    for (let i = 0; i < 25; i++) {
      calculator.processMessage(noviceMessages[i % noviceMessages.length]);
      calculator.processToolCall('read_file', true);
      if (i % 5 === 0) calculator.processCode(noviceCode);
    }

    calculator.recalculate(true);
    const level = calculator.getLevel();

    // Should detect as Novice or Apprentice
    assert.ok(level <= ProfileLevel.APPRENTICE);
  });

  it('detects expert profile organically', () => {
    const calculator = createProfileCalculator();

    // Simulate expert behavior
    const expertMessages = [
      'What are the trade-offs between CQRS and traditional architecture?',
      'How should I implement event sourcing with Kafka?',
      'What\'s the best approach for distributed transactions?',
      'How do I optimize this GraphQL resolver for N+1 queries?',
      'Should I use Kubernetes or ECS for this microservices deployment?',
    ];

    const expertCode = `
      @Injectable()
      class EventSourcedRepository<T extends AggregateRoot> implements IRepository<T> {
        constructor(
          private readonly eventStore: IEventStore,
          private readonly snapshotStore: ISnapshotStore<T>,
        ) {}

        async findById(id: string): Promise<Result<T, NotFoundError>> {
          try {
            const snapshot = await this.snapshotStore.get(id);
            const events = await this.eventStore.getEvents(id, snapshot?.version);
            return Result.ok(this.hydrate(snapshot, events));
          } catch (error: DomainError) {
            console.error('Repository error:', error);
            return Result.err(new NotFoundError(id));
          }
        }
      }
    `;

    for (let i = 0; i < 25; i++) {
      calculator.processMessage(expertMessages[i % expertMessages.length]);
      calculator.processToolCall('architect', true);
      calculator.processToolCall('review', true);
      if (i % 3 === 0) calculator.processCode(expertCode);
    }

    calculator.recalculate(true);
    const level = calculator.getLevel();

    // Should detect as Expert or Master
    assert.ok(level >= ProfileLevel.EXPERT);
  });

  it('detects practitioner profile organically', () => {
    const calculator = createProfileCalculator();

    // Simulate practitioner behavior (balanced)
    const practitionerMessages = [
      'How do I implement authentication in Express?',
      'What\'s the best way to handle errors in async functions?',
      'How do I write unit tests for this service?',
      'Should I use TypeScript or JavaScript for this project?',
    ];

    const practitionerCode = `
      class UserService {
        constructor(repository) {
          this.repository = repository;
        }

        async getUser(id) {
          try {
            return await this.repository.findById(id);
          } catch (error) {
            console.error('Failed to get user:', error);
            throw error;
          }
        }
      }

      describe('UserService', () => {
        it('gets user', async () => {
          const user = await service.getUser('1');
          expect(user).toBeDefined();
        });
      });
    `;

    for (let i = 0; i < 25; i++) {
      calculator.processMessage(practitionerMessages[i % practitionerMessages.length]);
      calculator.processToolCall('edit', true);
      calculator.processToolCall('grep', true);
      if (i % 4 === 0) calculator.processCode(practitionerCode);
    }

    calculator.recalculate(true);
    const level = calculator.getLevel();

    // Should detect as Practitioner (level 3)
    assert.ok(level >= ProfileLevel.APPRENTICE && level <= ProfileLevel.EXPERT);
  });
});
