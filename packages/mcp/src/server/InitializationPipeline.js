/**
 * InitializationPipeline - Post-ServiceInitializer Setup
 *
 * Handles all initialization that happens AFTER ServiceInitializer creates
 * core services: collective state restoration, orchestrator wiring,
 * Q-Learning, LLM routing, Solana anchoring, privacy stores, Oracle, etc.
 *
 * "L'éveil du chien" - κυνικός
 *
 * @module @cynic/mcp/server/InitializationPipeline
 */

'use strict';

import { join } from 'path';
import { fileURLToPath } from 'url';
import { FibonacciIntervals, EcosystemMonitor, globalEventBus } from '@cynic/core';
import { Tracer, createPhiSampler, createTracingMiddleware } from '@cynic/core/tracing';
import { getCollectivePackAsync, getQLearningServiceSingleton, initializeQLearning, createAutonomousDaemon, createHeartbeatService, createSLATracker, createConsciousnessBridge, createDefaultChecks, createEmergenceDetector } from '@cynic/node';
import { traceAllDogs } from '@cynic/node/tracing/dog-tracing.js';
import { TraceStorage } from '@cynic/persistence/services/trace-storage';
import { AnchorQueue, SolanaAnchorer, loadWalletFromFile, loadWalletFromEnv, SolanaCluster } from '@cynic/anchor';
import { BlockchainBridge } from '../blockchain-bridge.js';
import { AuthService } from '../auth-service.js';
import { XProxyService } from '../services/x-proxy.js';
import { LocalXStore, LocalPrivacyStore } from '@cynic/persistence';
import { createAllTools } from '../tools/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');

export class InitializationPipeline {
  /**
   * @param {Object} options
   * @param {Object} options.server - MCPServer instance (dependency container)
   */
  constructor({ server }) {
    this._server = server;
  }

  /**
   * Run the full initialization pipeline
   */
  async run() {
    const s = this._server;

    await this._restoreCollectiveState();
    this._initializeTracing();
    this._wireOrchestrators();
    await this._initializeQLearning();
    await this._initializeLLMRouter();
    await this._initializePerceptionRouter();
    await this._initializeSolanaAnchoring();
    this._initializeEcosystemMonitor();
    this._registerSchedulerTasks();
    await this._initializeBlockchainBridge();
    await this._initializeAutonomousDaemon();
    await this._initializeObservabilityStack();
    await this._initializeEmergenceDetector();
    this._initializeAuth();
    await this._initializeLocalStores();
    await this._initializeXProxy();
    await this._initializeOracle();
    this._registerTools();
  }

  /**
   * Restore collective state from persistence
   * @private
   */
  async _restoreCollectiveState() {
    const s = this._server;

    if (s.persistence) {
      try {
        // Re-initialize collectivePack with async persistence loading
        await getCollectivePackAsync({
          sharedMemory: s.sharedMemory,
          judge: s.judge,
          persistence: s.persistence,
          consensusThreshold: 0.618,
        });
        console.error('   CollectivePack: state restored from persistence');
        s._collectiveReady = true;
      } catch (e) {
        console.error(`   CollectivePack: persistence load failed (${e.message})`);
        s._collectiveReady = true; // Still ready, just without persistence
      }

      // Initialize LearningService from persistence
      if (s.learningService?.init) {
        try {
          await s.learningService.init();
          console.error('   LearningService: initialized from persistence');
        } catch (e) {
          console.error(`   LearningService: init failed (${e.message})`);
        }
      }
    } else {
      s._collectiveReady = true;
    }
  }

  /**
   * Initialize distributed tracing: Tracer → EventBus middleware → Dog wrapping → Storage
   * @private
   */
  _initializeTracing() {
    const s = this._server;

    try {
      // Create storage backend (uses PostgreSQL pool if available)
      const pool = s.persistence?.postgres?.pool || null;
      s.traceStorage = new TraceStorage({ pool });

      // Create tracer with φ-aligned sampling (10% default, max 61.8%)
      s.tracer = new Tracer({
        serviceName: 'cynic-mcp',
        sampler: createPhiSampler(0.1),
        storage: s.traceStorage,
      });

      // Wire tracing middleware into globalEventBus
      globalEventBus.use(createTracingMiddleware(s.tracer));

      // Wrap all Dogs with tracing spans
      if (s.collectivePack?.agents) {
        traceAllDogs(s.collectivePack.agents, s.tracer);
      }

      const storageType = pool ? 'PostgreSQL' : 'buffer-only';
      console.error(`   Tracing: ACTIVE (10% sampling, ${storageType})`);
    } catch (err) {
      console.error(`   Tracing: FAILED (${err.message})`);
      s.tracer = null;
      s.traceStorage = null;
    }
  }

  /**
   * Wire UnifiedOrchestrator and KabbalisticRouter with persistence
   * @private
   */
  _wireOrchestrators() {
    const s = this._server;

    // Wire UnifiedOrchestrator
    if (s.unifiedOrchestrator && s.persistence) {
      s.unifiedOrchestrator.persistence = s.persistence;
      if (s.persistence.memoryRetriever) {
        s.unifiedOrchestrator.memoryRetriever = s.persistence.memoryRetriever;
      }
      if (s.persistence.psychology) {
        s.unifiedOrchestrator.psychologyProvider = s.persistence.psychology;
      }
      console.error('   UnifiedOrchestrator: wired with persistence');
    }

    // Wire KabbalisticRouter
    if (s.kabbalisticRouter) {
      if (s.persistence) {
        s.kabbalisticRouter.persistence = s.persistence;
      }
      if (s.collectivePack && !s.kabbalisticRouter.collectivePack) {
        s.kabbalisticRouter.collectivePack = s.collectivePack;
      }
      if (s.learningService) {
        s.kabbalisticRouter.setLearningService?.(s.learningService);
      }
      if (s.costOptimizer) {
        s.kabbalisticRouter.setCostOptimizer?.(s.costOptimizer);
      }
      console.error('   KabbalisticRouter: wired (Lightning Flash active)');
    }
  }

  /**
   * Initialize Q-Learning from PostgreSQL
   * @private
   */
  async _initializeQLearning() {
    const s = this._server;

    if (!s.persistence) return;

    try {
      const loaded = await initializeQLearning(s.persistence);
      if (loaded) {
        console.error('   Q-Learning: state loaded from PostgreSQL');
      } else {
        console.error('   Q-Learning: initialized (fresh state)');
      }
    } catch (e) {
      console.error(`   Q-Learning: init failed (${e.message})`);
    }

    // Bridge LearningService to Q-Learning for routing optimization
    if (s.learningService) {
      try {
        const qService = getQLearningServiceSingleton();
        s.learningService.setQLearningService?.(qService);
        console.error('   Learning->Q-Learning bridge: active');
      } catch (e) {
        console.error(`   Learning->Q-Learning bridge: failed (${e.message})`);
      }
    }
  }

  /**
   * Initialize multi-model LLM routing
   * @private
   */
  async _initializeLLMRouter() {
    const s = this._server;

    if (!s.llmRouter) {
      try {
        const { getRouterWithValidators } = await import('@cynic/llm');
        s.llmRouter = getRouterWithValidators();
        if (s.unifiedOrchestrator) {
          s.unifiedOrchestrator.setLLMRouter(s.llmRouter);
        }
        const validatorCount = s.llmRouter.validators?.length || 0;
        console.error(`   LLMRouter: active (${validatorCount} validators)`);
      } catch (e) {
        console.error(`   LLMRouter: unavailable (${e.message})`);
      }
    } else if (s.unifiedOrchestrator) {
      s.unifiedOrchestrator.setLLMRouter(s.llmRouter);
    }
  }

  /**
   * Initialize perception routing
   * @private
   */
  async _initializePerceptionRouter() {
    const s = this._server;

    if (!s.perceptionRouter) {
      try {
        const { getPerceptionRouter } = await import('@cynic/llm');
        s.perceptionRouter = getPerceptionRouter();
        console.error('   PerceptionRouter: active');
      } catch (e) {
        console.error(`   PerceptionRouter: unavailable (${e.message})`);
      }
    }

    // Wire PerceptionRouter to UnifiedOrchestrator
    if (s.perceptionRouter && s.unifiedOrchestrator) {
      s.unifiedOrchestrator.setPerceptionRouter(s.perceptionRouter);
      console.error('   UnifiedOrchestrator: wired with PerceptionRouter');
    }
  }

  /**
   * Initialize Solana anchoring if configured
   * @private
   */
  async _initializeSolanaAnchoring() {
    const s = this._server;
    const walletPath = process.env.CYNIC_SOLANA_WALLET || join(__dirname, '../../../anchor/test/.devnet-wallet.json');
    const enableAnchoring = process.env.CYNIC_ENABLE_ANCHORING === 'true';

    if (enableAnchoring) {
      try {
        const wallet = loadWalletFromEnv('CYNIC_SOLANA_KEY') || loadWalletFromFile(walletPath);
        const cluster = process.env.CYNIC_SOLANA_CLUSTER || 'devnet';
        const clusterUrl = SolanaCluster[cluster.toUpperCase()] || SolanaCluster.DEVNET;

        // Create Solana anchorer with wallet
        const anchorer = new SolanaAnchorer({
          cluster: clusterUrl,
          wallet,
          useAnchorProgram: true,
        });

        // Create anchor queue with anchorer
        s.anchorQueue = new AnchorQueue({
          anchorer,
          batchSize: 100,
          intervalMs: 61800,
          autoStart: true,
        });

        if (s.pojChainManager) {
          s.pojChainManager.setAnchorQueue(s.anchorQueue);
          console.error(`   Solana Anchoring: ENABLED (${cluster})`);
          console.error(`   AnchorQueue attached: ${s.pojChainManager.isAnchoringEnabled}`);
        } else {
          console.error('   Solana Anchoring: FAILED - pojChainManager not available');
        }
        const pubKeyStr = wallet.publicKey?.toBase58
          ? wallet.publicKey.toBase58()
          : Buffer.from(wallet.publicKey || wallet._publicKey || []).toString('hex');
        console.error(`   Wallet: ${pubKeyStr.slice(0, 16)}...`);
      } catch (err) {
        console.error(`   Solana Anchoring: DISABLED (${err.message})`);
      }
    } else {
      console.error('   Solana Anchoring: disabled (set CYNIC_ENABLE_ANCHORING=true)');
    }
  }

  /**
   * Initialize Ecosystem Monitor for GitHub tracking
   * @private
   */
  _initializeEcosystemMonitor() {
    const s = this._server;

    if (!s.ecosystemMonitor) {
      s.ecosystemMonitor = new EcosystemMonitor({
        maxCacheSize: 100,
        onUpdate: (updates, source) => {
          if (updates.length > 0) {
            s._broadcastSSEEvent('ecosystem_updates', {
              count: updates.length,
              source: source?.name || 'all',
              highPriority: updates.filter(u => u.priority === 'HIGH' || u.priority === 'CRITICAL').length,
              timestamp: Date.now(),
            });
          }
        },
      });
      s.ecosystemMonitor.registerSolanaDefaults();
      s.ecosystemMonitor.trackGitHubRepo('anza-xyz', 'agave', {
        trackReleases: true,
        trackCommits: false,
      });
      console.error(`   EcosystemMonitor: ready (${s.ecosystemMonitor.sources.size} sources)`);
    }
  }

  /**
   * Register scheduler tasks
   * @private
   */
  _registerSchedulerTasks() {
    const s = this._server;

    if (!s.scheduler) return;

    // Psychology checkpoint - sync every 10 minutes to prevent data loss on crash
    s.scheduler.register({
      id: 'psychology_checkpoint',
      name: 'Psychology Checkpoint',
      intervalMs: 10 * 60 * 1000, // 10 minutes
      runImmediately: false,
      handler: async () => {
        if (!s.persistence?.psychology || !s.sessionManager?._currentSession) {
          return { skipped: true, reason: 'no_active_session' };
        }

        const session = s.sessionManager._currentSession;
        const userId = session.userId;

        if (!userId) {
          return { skipped: true, reason: 'no_user_id' };
        }

        try {
          let psychologyData = null;

          if (s.collective?.cynic) {
            const cynicStats = s.collective.cynic.getStatus?.() || {};
            psychologyData = {
              sessionId: session.sessionId,
              checkpoint: true,
              timestamp: Date.now(),
              observedEvents: cynicStats.observedEvents || 0,
              synthesizedPatterns: cynicStats.synthesizedPatterns || 0,
              decisions: cynicStats.decisions || 0,
            };
          }

          if (psychologyData) {
            await s.persistence.syncPsychology(userId, psychologyData);
            console.error(`[CHECKPOINT] Psychology synced for ${userId.slice(0, 8)}...`);
            return { synced: true, userId: userId.slice(0, 8) };
          }

          return { skipped: true, reason: 'no_psychology_data' };
        } catch (err) {
          console.error(`[CHECKPOINT] Psychology sync failed: ${err.message}`);
          return { error: err.message };
        }
      },
    });

    console.error('   Psychology checkpoint: every 10 minutes');

    s.scheduler.register({
      id: 'ecosystem_awareness',
      name: 'Ecosystem Awareness',
      intervalMs: FibonacciIntervals.SIXHOURLY,
      runImmediately: false,
      handler: async () => {
        console.error('[Scheduler] Starting ecosystem awareness cycle...');

        const fetchResult = await s.ecosystemMonitor.fetchAll();
        const highPriorityUpdates = (fetchResult.updates || [])
          .filter(u => u.priority === 'HIGH' || u.priority === 'CRITICAL');

        let judgedCount = 0;
        if (highPriorityUpdates.length > 0 && s.graphIntegration) {
          for (const update of highPriorityUpdates.slice(0, 5)) {
            try {
              const item = {
                type: 'ecosystem_update',
                content: `${update.type}: ${update.title || 'Update'}\n\n${update.description || ''}`.slice(0, 500),
                source: update.source,
                sources: [update.url].filter(Boolean),
                priority: update.priority,
                meta: update.meta,
              };
              await s.graphIntegration.judgeWithGraph(item, { source: 'ecosystem_monitor' });
              judgedCount++;
            } catch (e) {
              console.error(`[Scheduler] Judge error: ${e.message}`);
            }
          }
          if (judgedCount > 0) {
            console.error(`[Scheduler] Judged ${judgedCount} high-priority updates`);
          }
        }

        s._broadcastSSEEvent('ecosystem_cycle', {
          fetched: fetchResult.fetched || 0,
          skipped: fetchResult.skipped || 0,
          errors: fetchResult.errors || 0,
          updatesFound: fetchResult.updates?.length || 0,
          highPriority: highPriorityUpdates.length,
          judged: judgedCount,
          timestamp: Date.now(),
        });

        console.error(`[Scheduler] Ecosystem cycle complete: ${fetchResult.updates?.length || 0} updates, ${judgedCount} judged`);
        return { fetched: fetchResult.fetched || 0, updatesFound: fetchResult.updates?.length || 0, highPriority: highPriorityUpdates.length, judged: judgedCount };
      },
    });

    console.error(`   Scheduler: ready (ecosystem awareness every ${FibonacciIntervals.SIXHOURLY / 3600000}h)`);
  }

  /**
   * Initialize BlockchainBridge
   * @private
   */
  async _initializeBlockchainBridge() {
    const s = this._server;

    if (!s.blockchainBridge) {
      s.blockchainBridge = new BlockchainBridge({
        eScore: s.eScoreCalculator,
        collective: s.collective,
        persistence: s.persistence,
        burnVerifier: s.burnVerifier,
      });
      s.blockchainBridge.start();
      console.error('   BlockchainBridge: active (PoJ -> Anchor -> E-Score loop)');
    }
  }

  /**
   * Initialize autonomous daemon
   * @private
   */
  async _initializeAutonomousDaemon() {
    const s = this._server;

    if (!s.autonomousDaemon && s.persistence?.pool) {
      try {
        s.autonomousDaemon = createAutonomousDaemon({
          pool: s.persistence.pool,
          memoryRetriever: s.persistence?.repositories?.memory,
          collective: s.collective,
          goalsRepo: s.persistence?.repositories?.autonomousGoals,
          tasksRepo: s.persistence?.repositories?.autonomousTasks,
          notificationsRepo: s.persistence?.repositories?.proactiveNotifications,
        });
        await s.autonomousDaemon.start();
        console.error('   AutonomousDaemon: ACTIVE (Fibonacci intervals, autonomous task processing)');
      } catch (err) {
        console.error(`   AutonomousDaemon: FAILED (${err.message})`);
        s.autonomousDaemon = null;
      }
    }
  }

  /**
   * Initialize observability stack (HeartbeatService, SLATracker, ConsciousnessBridge)
   * @private
   */
  async _initializeObservabilityStack() {
    const s = this._server;

    if (!s.heartbeatService && s.persistence?.pool) {
      try {
        const healthChecks = createDefaultChecks({
          pool: s.persistence.pool,
          redis: s.persistence.redis,
          mcpUrl: s.mode === 'http' ? `http://localhost:${s.port}` : null,
          collectivePack: s.collectivePack,
        });

        s.heartbeatService = createHeartbeatService({
          components: healthChecks,
          config: {
            intervalMs: 30000,
            timeoutMs: 5000,
          },
        });

        s.slaTracker = createSLATracker({
          heartbeat: s.heartbeatService,
          alertManager: {
            critical: (msg, data) => console.error(`SLA CRITICAL: ${msg}`, data),
            warning: (msg, data) => console.error(`SLA WARNING: ${msg}`, data),
          },
        });

        s.consciousnessBridge = createConsciousnessBridge({
          consciousness: s.collectivePack?.consciousness || null,
          heartbeat: s.heartbeatService,
          slaTracker: s.slaTracker,
        });

        s.heartbeatService.start();
        console.error('   HeartbeatService: ACTIVE (30s interval)');
        console.error('   SLATracker: ACTIVE (99.9% uptime target)');
        console.error('   ConsciousnessBridge: ACTIVE (health -> awareness)');
      } catch (err) {
        console.error(`   ObservabilityStack: FAILED (${err.message})`);
      }
    }
  }

  /**
   * Initialize cross-session pattern detection
   * @private
   */
  async _initializeEmergenceDetector() {
    const s = this._server;

    if (!s.emergenceDetector && s.persistence?.pool) {
      try {
        s.emergenceDetector = createEmergenceDetector({
          persistence: s.persistence,
          memoryRetriever: s.persistence?.repositories?.memory,
          embedder: s.persistence?.embedder || null,
          config: {
            analysisIntervalMs: 60 * 60 * 1000, // 1 hour
          },
        });

        // Wire emergence events to consciousness
        s.emergenceDetector.on('significant_pattern', (data) => {
          console.error(`EMERGENCE: ${data.pattern.subject} (${data.pattern.significance})`);
          if (s.consciousnessBridge) {
            s.consciousnessBridge.observe('EMERGENCE', {
              pattern: data.pattern.key,
              significance: data.pattern.significance,
              category: data.pattern.category,
            }, data.pattern.confidence);
          }
        });

        s.emergenceDetector.start();
        console.error('   EmergenceDetector: ACTIVE (1h analysis interval)');
      } catch (err) {
        console.error(`   EmergenceDetector: FAILED (${err.message})`);
      }
    }
  }

  /**
   * Initialize Auth service for HTTP mode
   * @private
   */
  _initializeAuth() {
    const s = this._server;

    if (s.mode === 'http' && !s.auth) {
      s.auth = new AuthService({
        publicPaths: ['/', '/health', '/metrics', '/dashboard', '/sse', '/api', '/mcp', '/message'],
      });
      const authStatus = s.auth.required ? 'required' : 'optional (dev mode)';
      console.error(`   Auth: ${authStatus} (${s.auth.apiKeys.size} keys configured)`);
    }
  }

  /**
   * Initialize local privacy stores (SQLite)
   * @private
   */
  async _initializeLocalStores() {
    const s = this._server;

    // Initialize LocalXStore (for X/Twitter data - always local first)
    if (!s.localXStore) {
      try {
        s.localXStore = new LocalXStore({
          verbose: process.env.CYNIC_DEBUG === 'true',
        });
        await s.localXStore.initialize();
        console.error(`   LocalXStore: ready (${s.localXStore.dbPath})`);
      } catch (err) {
        console.error(`   LocalXStore: FAILED (${err.message})`);
        s.localXStore = null;
      }
    }

    // Initialize LocalPrivacyStore
    if (!s.localPrivacyStore) {
      try {
        s.localPrivacyStore = new LocalPrivacyStore({
          userId: s.sessionManager?._currentSession?.userId || 'default',
          verbose: process.env.CYNIC_DEBUG === 'true',
        });
        await s.localPrivacyStore.initialize();
        console.error(`   LocalPrivacyStore: ready (${s.localPrivacyStore.dbPath})`);
      } catch (err) {
        console.error(`   LocalPrivacyStore: FAILED (${err.message})`);
        s.localPrivacyStore = null;
      }
    }
  }

  /**
   * Initialize X/Twitter proxy service
   * @private
   */
  async _initializeXProxy() {
    const s = this._server;

    if (process.env.CYNIC_X_PROXY_ENABLED === 'true' && !s.xProxy) {
      if (s.localXStore) {
        s.xProxy = new XProxyService({
          port: parseInt(process.env.CYNIC_X_PROXY_PORT || '8888'),
          localStore: s.localXStore,
          xRepository: s.persistence?.repositories?.xData,
          sslCaDir: process.env.CYNIC_X_PROXY_CERT_PATH || './.cynic-proxy-certs',
          verbose: process.env.CYNIC_X_PROXY_VERBOSE === 'true',
        });

        try {
          await Promise.race([
            s.xProxy.start(),
            new Promise((_, reject) => setTimeout(() => reject(new Error('start timeout (5s)')), 5000)),
          ]);
          console.error(`   X Proxy: ENABLED (port ${s.xProxy.port}) - LOCAL FIRST`);
          console.error(`   Configure browser/system proxy: 127.0.0.1:${s.xProxy.port}`);
        } catch (err) {
          console.error(`   X Proxy: FAILED (${err.message})`);
          s.xProxy = null;
        }
      } else {
        console.error('   X Proxy: DISABLED (LocalXStore not available)');
      }
    }
  }

  /**
   * Initialize Oracle (token scoring)
   * @private
   */
  async _initializeOracle() {
    const s = this._server;

    if (!s.oracle) {
      try {
        const { TokenFetcher, TokenScorer, OracleMemory, OracleWatchlist } =
          await import('@cynic/observatory/oracle');
        const fetcher = new TokenFetcher(process.env.HELIUS_API_KEY);
        const scorer = new TokenScorer();
        const memory = s.persistence?.pool
          ? new OracleMemory(s.persistence.pool) : null;
        const watchlist = (memory && s.persistence?.pool)
          ? new OracleWatchlist(s.persistence.pool, memory, fetcher, scorer) : null;
        s.oracle = { fetcher, scorer, memory, watchlist };
        console.error(`   Oracle: ready (memory: ${!!memory}, watchlist: ${!!watchlist})`);
      } catch (e) {
        console.error(`   Oracle: unavailable (${e.message})`);
        s.oracle = null;
      }
    }
  }

  /**
   * Register all tools
   * @private
   */
  _registerTools() {
    const s = this._server;

    s.tools = createAllTools({
      judge: s.judge,
      node: s.node,
      persistence: s.persistence,
      collective: s.collective,
      sessionManager: s.sessionManager,
      pojChainManager: s.pojChainManager,
      librarian: s.librarian,
      ecosystem: s.ecosystem,
      integrator: s.integrator,
      metrics: s.metrics,
      graphIntegration: s.graphIntegration,
      discovery: s.discovery,
      learningService: s.learningService,
      eScoreCalculator: s.eScoreCalculator,
      emergenceLayer: s.emergenceLayer,
      patternDetector: s.patternDetector,
      thermodynamics: s.thermodynamics,
      onJudgment: (judgment) => s._broadcastSSEEvent('judgment', judgment),
      memoryRetriever: s.persistence?.memoryRetriever,
      goalsRepo: s.persistence?.goals,
      notificationsRepo: s.persistence?.notifications,
      tasksRepo: s.persistence?.tasks,
      automationExecutor: s.automationExecutor,
      heartbeat: s.heartbeatService,
      slaTracker: s.slaTracker,
      complexityClassifier: s.complexityClassifier,
      tieredRouter: s.tieredRouter,
      agentBooster: s.agentBooster,
      tokenOptimizer: s.tokenOptimizer,
      hyperbolicSpace: s.hyperbolicSpace,
      sona: s.sona,
      dogOrchestrator: s.dogOrchestrator,
      engineOrchestrator: s.engineOrchestrator,
      xRepository: s.persistence?.repositories?.xData,
      localXStore: s.localXStore,
      localPrivacyStore: s.localPrivacyStore,
      oracle: s.oracle,
      sharedMemory: s.sharedMemory,
      getQLearningService: getQLearningServiceSingleton,
    });

    // Feed registered tool names to PerceptionRouter for Layer 2 routing
    if (s.perceptionRouter && s.tools) {
      s.perceptionRouter.registerMcpTools(Object.keys(s.tools));
    }
  }
}
