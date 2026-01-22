/**
 * CYNIC Game Theory Engine
 *
 * "Strategic interaction and equilibrium"
 *
 * Philosophical foundations:
 * - Nash: Non-cooperative games, equilibrium concepts
 * - Schelling: Focal points, coordination
 * - Axelrod: Cooperation evolution, repeated games
 * - Binmore: Game theory and social contract
 *
 * φ guides all ratios: 61.8% confidence max
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

// ─────────────────────────────────────────────────────────────
// φ CONSTANTS
// ─────────────────────────────────────────────────────────────

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2% - uncertainty threshold
const PHI_INV_3 = 0.236067977499790;    // 23.6% - minimum threshold

// ─────────────────────────────────────────────────────────────
// GAME TYPES
// ─────────────────────────────────────────────────────────────

const GAME_TYPES = {
  // By player structure
  two_player: {
    name: 'Two-Player',
    minPlayers: 2,
    maxPlayers: 2,
    weight: PHI_INV
  },
  n_player: {
    name: 'N-Player',
    minPlayers: 3,
    maxPlayers: Infinity,
    weight: PHI_INV_2
  },

  // By information
  perfect_info: {
    name: 'Perfect Information',
    description: 'All players know all moves',
    weight: PHI_INV
  },
  imperfect_info: {
    name: 'Imperfect Information',
    description: 'Some information hidden',
    weight: PHI_INV + PHI_INV_3
  },
  incomplete_info: {
    name: 'Incomplete Information',
    description: 'Unknown player types/payoffs',
    weight: PHI_INV_2
  },

  // By sum
  zero_sum: {
    name: 'Zero-Sum',
    description: 'One\'s gain is another\'s loss',
    weight: PHI_INV_2
  },
  non_zero_sum: {
    name: 'Non-Zero-Sum',
    description: 'Mutual gain/loss possible',
    weight: PHI_INV
  },

  // By timing
  simultaneous: {
    name: 'Simultaneous',
    description: 'Players move at same time',
    weight: PHI_INV_2
  },
  sequential: {
    name: 'Sequential',
    description: 'Players move in order',
    weight: PHI_INV
  },

  // By repetition
  one_shot: {
    name: 'One-Shot',
    description: 'Single interaction',
    weight: PHI_INV_2
  },
  repeated: {
    name: 'Repeated',
    description: 'Multiple interactions',
    weight: PHI_INV
  },
  indefinite: {
    name: 'Indefinitely Repeated',
    description: 'Unknown end point',
    weight: PHI_INV + PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// CLASSIC GAMES (Templates)
// ─────────────────────────────────────────────────────────────

const CLASSIC_GAMES = {
  prisoners_dilemma: {
    name: 'Prisoner\'s Dilemma',
    players: ['A', 'B'],
    strategies: ['cooperate', 'defect'],
    payoffs: {
      'cooperate,cooperate': [3, 3],
      'cooperate,defect': [0, 5],
      'defect,cooperate': [5, 0],
      'defect,defect': [1, 1]
    },
    nashEquilibria: [['defect', 'defect']],
    paretoOptimal: [['cooperate', 'cooperate']],
    dilemma: true,
    insight: 'Individual rationality leads to collective irrationality'
  },

  stag_hunt: {
    name: 'Stag Hunt',
    players: ['A', 'B'],
    strategies: ['stag', 'hare'],
    payoffs: {
      'stag,stag': [4, 4],
      'stag,hare': [0, 3],
      'hare,stag': [3, 0],
      'hare,hare': [3, 3]
    },
    nashEquilibria: [['stag', 'stag'], ['hare', 'hare']],
    paretoOptimal: [['stag', 'stag']],
    coordinationProblem: true,
    insight: 'Trust required for optimal outcome'
  },

  battle_of_sexes: {
    name: 'Battle of the Sexes',
    players: ['A', 'B'],
    strategies: ['opera', 'football'],
    payoffs: {
      'opera,opera': [3, 2],
      'opera,football': [0, 0],
      'football,opera': [0, 0],
      'football,football': [2, 3]
    },
    nashEquilibria: [['opera', 'opera'], ['football', 'football']],
    coordinationProblem: true,
    insight: 'Coordination with conflicting preferences'
  },

  chicken: {
    name: 'Chicken (Hawk-Dove)',
    players: ['A', 'B'],
    strategies: ['swerve', 'straight'],
    payoffs: {
      'swerve,swerve': [3, 3],
      'swerve,straight': [1, 4],
      'straight,swerve': [4, 1],
      'straight,straight': [0, 0]
    },
    nashEquilibria: [['swerve', 'straight'], ['straight', 'swerve']],
    anticoordinationGame: true,
    insight: 'Credible commitment can be advantageous'
  },

  matching_pennies: {
    name: 'Matching Pennies',
    players: ['Matcher', 'Mismatcher'],
    strategies: ['heads', 'tails'],
    payoffs: {
      'heads,heads': [1, -1],
      'heads,tails': [-1, 1],
      'tails,heads': [-1, 1],
      'tails,tails': [1, -1]
    },
    nashEquilibria: [], // Only mixed strategy equilibrium
    mixedEquilibrium: [[0.5, 0.5], [0.5, 0.5]],
    zeroSum: true,
    insight: 'No pure strategy equilibrium exists'
  },

  public_goods: {
    name: 'Public Goods Game',
    players: ['1', '2', '...', 'n'],
    strategies: ['contribute', 'free_ride'],
    socialDilemma: true,
    insight: 'Free-riding undermines collective benefit'
  },

  ultimatum: {
    name: 'Ultimatum Game',
    players: ['Proposer', 'Responder'],
    sequential: true,
    insight: 'Fairness norms override pure rationality'
  },

  dictator: {
    name: 'Dictator Game',
    players: ['Dictator', 'Recipient'],
    sequential: true,
    insight: 'Pure altruism measurement'
  }
};

// ─────────────────────────────────────────────────────────────
// EQUILIBRIUM CONCEPTS
// ─────────────────────────────────────────────────────────────

const EQUILIBRIUM_TYPES = {
  nash: {
    name: 'Nash Equilibrium',
    description: 'No player can unilaterally improve',
    refinement: false,
    strength: PHI_INV
  },
  dominant_strategy: {
    name: 'Dominant Strategy Equilibrium',
    description: 'Each player has a dominant strategy',
    refinement: false,
    strength: PHI_INV + PHI_INV_3
  },
  subgame_perfect: {
    name: 'Subgame Perfect Equilibrium',
    description: 'Nash in every subgame',
    refinement: true,
    strength: PHI_INV
  },
  trembling_hand: {
    name: 'Trembling Hand Perfect',
    description: 'Robust to small mistakes',
    refinement: true,
    strength: PHI_INV_2
  },
  bayesian: {
    name: 'Bayesian Nash Equilibrium',
    description: 'Equilibrium with incomplete info',
    refinement: false,
    strength: PHI_INV_2
  },
  correlated: {
    name: 'Correlated Equilibrium',
    description: 'Coordination via signals',
    refinement: false,
    strength: PHI_INV
  },
  evolutionary_stable: {
    name: 'Evolutionarily Stable Strategy',
    description: 'Resistant to mutant invasion',
    refinement: false,
    strength: PHI_INV
  }
};

// ─────────────────────────────────────────────────────────────
// COOPERATION STRATEGIES (Axelrod)
// ─────────────────────────────────────────────────────────────

const COOPERATION_STRATEGIES = {
  always_cooperate: {
    name: 'Always Cooperate',
    description: 'Unconditional cooperation',
    nice: true,
    forgiving: true,
    exploitable: true
  },
  always_defect: {
    name: 'Always Defect',
    description: 'Unconditional defection',
    nice: false,
    forgiving: false,
    exploitable: false
  },
  tit_for_tat: {
    name: 'Tit for Tat',
    description: 'Cooperate first, then mirror',
    nice: true,
    forgiving: true,
    retaliatory: true,
    clear: true,
    axelrodWinner: true
  },
  generous_tft: {
    name: 'Generous Tit for Tat',
    description: 'TFT but sometimes forgives',
    nice: true,
    forgiving: true,
    forgivenessRate: PHI_INV_3
  },
  grim_trigger: {
    name: 'Grim Trigger',
    description: 'Cooperate until betrayed, then always defect',
    nice: true,
    forgiving: false,
    retaliatory: true
  },
  pavlov: {
    name: 'Pavlov (Win-Stay, Lose-Shift)',
    description: 'Repeat if payoff good, switch if bad',
    nice: true,
    adaptive: true
  },
  random: {
    name: 'Random',
    description: 'Random choice each round',
    nice: false,
    unpredictable: true
  }
};

// ─────────────────────────────────────────────────────────────
// SCHELLING FOCAL POINTS
// ─────────────────────────────────────────────────────────────

const FOCAL_POINT_TYPES = {
  prominence: {
    name: 'Prominence',
    description: 'Stands out perceptually',
    weight: PHI_INV
  },
  uniqueness: {
    name: 'Uniqueness',
    description: 'Only option with property',
    weight: PHI_INV + PHI_INV_3
  },
  precedent: {
    name: 'Precedent',
    description: 'Historical pattern',
    weight: PHI_INV
  },
  convention: {
    name: 'Convention',
    description: 'Social norm',
    weight: PHI_INV + PHI_INV_3
  },
  symmetry: {
    name: 'Symmetry',
    description: 'Equal/fair division',
    weight: PHI_INV_2
  },
  natural: {
    name: 'Natural',
    description: 'Inherently salient',
    weight: PHI_INV
  }
};

// ─────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────

const state = {
  games: new Map(),           // Defined games
  interactions: [],           // Recorded interactions
  strategies: new Map(),      // Agent strategies
  equilibria: [],             // Found equilibria
  focalPoints: [],            // Identified focal points
  repeatedGames: new Map(),   // Repeated game histories
  stats: {
    gamesCreated: 0,
    interactionsRecorded: 0,
    equilibriaFound: 0,
    cooperationRate: 0,
    focalPointsIdentified: 0
  }
};

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'game-theory');
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

// ─────────────────────────────────────────────────────────────
// CORE FUNCTIONS
// ─────────────────────────────────────────────────────────────

/**
 * Define a new game
 */
function defineGame(id, spec) {
  const game = {
    id,
    name: spec.name || id,
    players: spec.players || ['A', 'B'],
    strategies: spec.strategies || {},
    payoffs: spec.payoffs || {},
    type: spec.type || 'simultaneous',
    information: spec.information || 'perfect_info',
    sum: spec.sum || 'non_zero_sum',
    created: Date.now(),
    template: spec.template || null // Reference to classic game
  };

  // If based on template, inherit properties
  if (spec.template && CLASSIC_GAMES[spec.template]) {
    const template = CLASSIC_GAMES[spec.template];
    game.strategies = game.strategies.length ? game.strategies : template.strategies;
    game.payoffs = Object.keys(game.payoffs).length ? game.payoffs : template.payoffs;
    game.insight = template.insight;
  }

  state.games.set(id, game);
  state.stats.gamesCreated++;

  appendHistory({
    type: 'game_defined',
    game: id,
    spec: game,
    timestamp: Date.now()
  });

  return game;
}

/**
 * Get payoff for strategy profile
 */
function getPayoff(gameId, strategyProfile) {
  const game = state.games.get(gameId);
  if (!game) return null;

  const key = Array.isArray(strategyProfile)
    ? strategyProfile.join(',')
    : strategyProfile;

  return game.payoffs[key] || null;
}

/**
 * Find Nash equilibria for a 2-player game
 */
function findNashEquilibria(gameId) {
  const game = state.games.get(gameId);
  if (!game || game.players.length !== 2) return [];

  const equilibria = [];
  const strategies = game.strategies;

  // Check all strategy combinations
  for (const s1 of strategies) {
    for (const s2 of strategies) {
      const profile = [s1, s2];
      const payoff = getPayoff(gameId, profile);

      if (!payoff) continue;

      // Check if player 1 can improve
      let p1CanImprove = false;
      for (const alt of strategies) {
        if (alt === s1) continue;
        const altPayoff = getPayoff(gameId, [alt, s2]);
        if (altPayoff && altPayoff[0] > payoff[0]) {
          p1CanImprove = true;
          break;
        }
      }

      // Check if player 2 can improve
      let p2CanImprove = false;
      for (const alt of strategies) {
        if (alt === s2) continue;
        const altPayoff = getPayoff(gameId, [s1, alt]);
        if (altPayoff && altPayoff[1] > payoff[1]) {
          p2CanImprove = true;
          break;
        }
      }

      // Nash equilibrium if neither can improve
      if (!p1CanImprove && !p2CanImprove) {
        equilibria.push({
          profile,
          payoff,
          type: 'nash',
          confidence: PHI_INV
        });
      }
    }
  }

  // Record finding
  if (equilibria.length > 0) {
    state.equilibria.push(...equilibria);
    state.stats.equilibriaFound += equilibria.length;
  }

  return equilibria;
}

/**
 * Check for dominant strategies
 */
function findDominantStrategies(gameId) {
  const game = state.games.get(gameId);
  if (!game || game.players.length !== 2) return { player1: null, player2: null };

  const strategies = game.strategies;
  const result = { player1: null, player2: null };

  // Check player 1 for dominant strategy
  for (const candidate of strategies) {
    let dominates = true;
    for (const other of strategies) {
      if (other === candidate) continue;

      // Must be better against ALL opponent strategies
      for (const opponentStrat of strategies) {
        const candidatePayoff = getPayoff(gameId, [candidate, opponentStrat]);
        const otherPayoff = getPayoff(gameId, [other, opponentStrat]);

        if (!candidatePayoff || !otherPayoff || candidatePayoff[0] <= otherPayoff[0]) {
          dominates = false;
          break;
        }
      }
      if (!dominates) break;
    }
    if (dominates) {
      result.player1 = candidate;
      break;
    }
  }

  // Check player 2 for dominant strategy
  for (const candidate of strategies) {
    let dominates = true;
    for (const other of strategies) {
      if (other === candidate) continue;

      for (const opponentStrat of strategies) {
        const candidatePayoff = getPayoff(gameId, [opponentStrat, candidate]);
        const otherPayoff = getPayoff(gameId, [opponentStrat, other]);

        if (!candidatePayoff || !otherPayoff || candidatePayoff[1] <= otherPayoff[1]) {
          dominates = false;
          break;
        }
      }
      if (!dominates) break;
    }
    if (dominates) {
      result.player2 = candidate;
      break;
    }
  }

  return result;
}

/**
 * Find Pareto optimal outcomes
 */
function findParetoOptimal(gameId) {
  const game = state.games.get(gameId);
  if (!game) return [];

  const strategies = game.strategies;
  const outcomes = [];

  // Collect all outcomes
  for (const s1 of strategies) {
    for (const s2 of strategies) {
      const payoff = getPayoff(gameId, [s1, s2]);
      if (payoff) {
        outcomes.push({ profile: [s1, s2], payoff });
      }
    }
  }

  // Find Pareto optimal (no outcome dominates it)
  const paretoOptimal = [];

  for (const outcome of outcomes) {
    let dominated = false;

    for (const other of outcomes) {
      if (outcome === other) continue;

      // other dominates outcome if better for at least one, no worse for all
      const atLeastOneBetter = other.payoff[0] > outcome.payoff[0] ||
                               other.payoff[1] > outcome.payoff[1];
      const noneWorse = other.payoff[0] >= outcome.payoff[0] &&
                        other.payoff[1] >= outcome.payoff[1];

      if (atLeastOneBetter && noneWorse) {
        dominated = true;
        break;
      }
    }

    if (!dominated) {
      paretoOptimal.push(outcome);
    }
  }

  return paretoOptimal;
}

/**
 * Record an interaction (play of a game)
 */
function recordInteraction(gameId, players, strategies, context = {}) {
  const game = state.games.get(gameId);

  const interaction = {
    id: `int_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    gameId,
    players,
    strategies,
    payoff: game ? getPayoff(gameId, strategies) : null,
    context,
    timestamp: Date.now()
  };

  state.interactions.push(interaction);
  state.stats.interactionsRecorded++;

  // Update cooperation tracking
  if (game && strategies.includes('cooperate')) {
    const cooperators = strategies.filter(s => s === 'cooperate').length;
    const total = strategies.length;
    state.stats.cooperationRate =
      (state.stats.cooperationRate * (state.stats.interactionsRecorded - 1) + cooperators / total) /
      state.stats.interactionsRecorded;
  }

  // Track repeated game history
  const repeatKey = `${gameId}:${players.sort().join(',')}`;
  if (!state.repeatedGames.has(repeatKey)) {
    state.repeatedGames.set(repeatKey, []);
  }
  state.repeatedGames.get(repeatKey).push(interaction);

  appendHistory({
    type: 'interaction',
    interaction,
    timestamp: Date.now()
  });

  return interaction;
}

/**
 * Register an agent's strategy for repeated games
 */
function registerStrategy(agentId, strategyType, params = {}) {
  const strategy = {
    agentId,
    type: strategyType,
    params,
    template: COOPERATION_STRATEGIES[strategyType] || null,
    registered: Date.now()
  };

  state.strategies.set(agentId, strategy);

  return strategy;
}

/**
 * Get next move for a strategy in repeated game
 */
function getStrategyMove(agentId, history) {
  const strategy = state.strategies.get(agentId);
  if (!strategy) return 'cooperate'; // Default

  const lastOpponentMove = history.length > 0
    ? history[history.length - 1].opponentMove
    : null;

  switch (strategy.type) {
    case 'always_cooperate':
      return 'cooperate';

    case 'always_defect':
      return 'defect';

    case 'tit_for_tat':
      return lastOpponentMove || 'cooperate';

    case 'generous_tft': {
      if (!lastOpponentMove) return 'cooperate';
      if (lastOpponentMove === 'cooperate') return 'cooperate';
      // Forgive with probability PHI_INV_3
      return Math.random() < PHI_INV_3 ? 'cooperate' : 'defect';
    }

    case 'grim_trigger': {
      const everDefected = history.some(h => h.opponentMove === 'defect');
      return everDefected ? 'defect' : 'cooperate';
    }

    case 'pavlov': {
      if (!lastOpponentMove) return 'cooperate';
      const lastMyMove = history[history.length - 1].myMove;
      const lastPayoff = history[history.length - 1].payoff;
      // Good payoff (>=3): repeat. Bad payoff: switch.
      return lastPayoff >= 3 ? lastMyMove : (lastMyMove === 'cooperate' ? 'defect' : 'cooperate');
    }

    case 'random':
      return Math.random() < 0.5 ? 'cooperate' : 'defect';

    default:
      return 'cooperate';
  }
}

/**
 * Identify focal point in coordination game
 */
function identifyFocalPoint(options, context = {}) {
  const scored = options.map(opt => {
    let score = 0;
    const reasons = [];

    // Prominence
    if (opt.prominent) {
      score += FOCAL_POINT_TYPES.prominence.weight;
      reasons.push('prominent');
    }

    // Uniqueness
    if (opt.unique) {
      score += FOCAL_POINT_TYPES.uniqueness.weight;
      reasons.push('unique');
    }

    // Precedent
    if (context.precedent === opt.value) {
      score += FOCAL_POINT_TYPES.precedent.weight;
      reasons.push('precedent');
    }

    // Convention
    if (opt.conventional) {
      score += FOCAL_POINT_TYPES.convention.weight;
      reasons.push('convention');
    }

    // Symmetry/Fairness
    if (opt.symmetric || opt.fair) {
      score += FOCAL_POINT_TYPES.symmetry.weight;
      reasons.push('symmetric');
    }

    // Natural salience
    if (opt.natural) {
      score += FOCAL_POINT_TYPES.natural.weight;
      reasons.push('natural');
    }

    return { option: opt, score: Math.min(score, PHI_INV), reasons };
  });

  // Sort by score
  scored.sort((a, b) => b.score - a.score);

  const focal = scored[0];

  if (focal && focal.score > PHI_INV_3) {
    state.focalPoints.push({
      option: focal.option,
      score: focal.score,
      reasons: focal.reasons,
      context,
      timestamp: Date.now()
    });
    state.stats.focalPointsIdentified++;
  }

  return {
    focalPoint: focal ? focal.option : null,
    confidence: focal ? focal.score : 0,
    reasons: focal ? focal.reasons : [],
    alternatives: scored.slice(1, 4)
  };
}

/**
 * Analyze social dilemma
 */
function analyzeDilemma(gameId) {
  const game = state.games.get(gameId);
  if (!game) return null;

  const nash = findNashEquilibria(gameId);
  const pareto = findParetoOptimal(gameId);

  // Dilemma exists if Nash is not Pareto optimal
  const nashProfiles = nash.map(n => n.profile.join(','));
  const paretoProfiles = pareto.map(p => p.profile.join(','));

  const dilemmaExists = nash.length > 0 &&
    !nash.every(n => paretoProfiles.includes(n.profile.join(',')));

  let dilemmaType = null;
  let insight = null;

  if (dilemmaExists) {
    // Classify type
    if (game.template === 'prisoners_dilemma' ||
        (nash.length === 1 && nash[0].profile.every(s => s === 'defect'))) {
      dilemmaType = 'prisoners_dilemma';
      insight = 'Individual rationality conflicts with collective welfare';
    } else if (nash.length > 1) {
      dilemmaType = 'coordination_failure';
      insight = 'Multiple equilibria create coordination risk';
    } else {
      dilemmaType = 'social_trap';
      insight = 'Short-term incentives undermine long-term benefit';
    }
  }

  return {
    gameId,
    dilemmaExists,
    dilemmaType,
    nashEquilibria: nash,
    paretoOptimal: pareto,
    insight,
    possibleResolutions: dilemmaExists ? [
      'Repeated interaction (shadow of future)',
      'Communication and commitment',
      'External enforcement',
      'Norm internalization',
      'Structural change to payoffs'
    ] : [],
    confidence: PHI_INV
  };
}

/**
 * Simulate repeated game
 */
function simulateRepeatedGame(gameId, strategy1, strategy2, rounds = 10) {
  const game = state.games.get(gameId);
  if (!game) return null;

  const history1 = [];
  const history2 = [];
  const results = [];

  // Temporary strategies
  const temp1 = `temp_${Date.now()}_1`;
  const temp2 = `temp_${Date.now()}_2`;
  registerStrategy(temp1, strategy1);
  registerStrategy(temp2, strategy2);

  let totalPayoff1 = 0;
  let totalPayoff2 = 0;

  for (let round = 0; round < rounds; round++) {
    const move1 = getStrategyMove(temp1, history1);
    const move2 = getStrategyMove(temp2, history2);

    const payoff = getPayoff(gameId, [move1, move2]);

    if (payoff) {
      totalPayoff1 += payoff[0];
      totalPayoff2 += payoff[1];

      history1.push({ myMove: move1, opponentMove: move2, payoff: payoff[0] });
      history2.push({ myMove: move2, opponentMove: move1, payoff: payoff[1] });

      results.push({
        round: round + 1,
        moves: [move1, move2],
        payoffs: payoff
      });
    }
  }

  // Cleanup temp strategies
  state.strategies.delete(temp1);
  state.strategies.delete(temp2);

  return {
    gameId,
    strategy1,
    strategy2,
    rounds,
    results,
    totalPayoffs: [totalPayoff1, totalPayoff2],
    averagePayoffs: [totalPayoff1 / rounds, totalPayoff2 / rounds],
    cooperationRates: [
      results.filter(r => r.moves[0] === 'cooperate').length / rounds,
      results.filter(r => r.moves[1] === 'cooperate').length / rounds
    ]
  };
}

// ─────────────────────────────────────────────────────────────
// ANALYSIS
// ─────────────────────────────────────────────────────────────

/**
 * Analyze strategic situation
 */
function analyzeStrategicSituation(situation) {
  const analysis = {
    situation,
    gameType: null,
    strategies: [],
    equilibriumPrediction: null,
    recommendations: [],
    confidence: PHI_INV_2
  };

  // Identify game type
  if (situation.zeroSum) {
    analysis.gameType = 'zero_sum';
    analysis.recommendations.push('Maximize minimum gain (minimax)');
  } else if (situation.cooperation_possible) {
    analysis.gameType = 'cooperation_game';
    analysis.recommendations.push('Build trust through repeated interaction');
    analysis.recommendations.push('Consider tit-for-tat or similar strategy');
  }

  if (situation.multipleEquilibria) {
    analysis.recommendations.push('Look for focal point');
    analysis.recommendations.push('Communicate if possible');
  }

  if (situation.asymmetricInfo) {
    analysis.gameType = 'incomplete_info';
    analysis.recommendations.push('Consider signaling strategies');
    analysis.recommendations.push('Account for adverse selection');
  }

  // Match to classic game
  for (const [key, game] of Object.entries(CLASSIC_GAMES)) {
    if (situation.dilemma && game.dilemma) {
      analysis.matchedGame = key;
      analysis.insight = game.insight;
      break;
    }
    if (situation.coordination && game.coordinationProblem) {
      analysis.matchedGame = key;
      analysis.insight = game.insight;
      break;
    }
  }

  return analysis;
}

// ─────────────────────────────────────────────────────────────
// PERSISTENCE
// ─────────────────────────────────────────────────────────────

function ensureStorageDir() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }
}

function saveState() {
  ensureStorageDir();

  const serializable = {
    games: Array.from(state.games.entries()),
    interactions: state.interactions.slice(-100),
    strategies: Array.from(state.strategies.entries()),
    equilibria: state.equilibria.slice(-50),
    focalPoints: state.focalPoints.slice(-30),
    repeatedGames: Array.from(state.repeatedGames.entries()),
    stats: state.stats
  };

  fs.writeFileSync(STATE_FILE, JSON.stringify(serializable, null, 2));
}

function loadState() {
  ensureStorageDir();

  if (fs.existsSync(STATE_FILE)) {
    try {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
      state.games = new Map(data.games || []);
      state.interactions = data.interactions || [];
      state.strategies = new Map(data.strategies || []);
      state.equilibria = data.equilibria || [];
      state.focalPoints = data.focalPoints || [];
      state.repeatedGames = new Map(data.repeatedGames || []);
      state.stats = data.stats || state.stats;
    } catch (e) {
      console.error('Failed to load game theory state:', e.message);
    }
  }

  // Initialize classic games
  for (const [key, game] of Object.entries(CLASSIC_GAMES)) {
    if (!state.games.has(key)) {
      defineGame(key, { ...game, template: key });
    }
  }
}

function appendHistory(entry) {
  ensureStorageDir();
  fs.appendFileSync(HISTORY_FILE, JSON.stringify(entry) + '\n');
}

// ─────────────────────────────────────────────────────────────
// FORMATTING
// ─────────────────────────────────────────────────────────────

function formatStatus() {
  const lines = [
    '── GAME THEORY ────────────────────────────────────────────',
    ''
  ];

  // Stats
  lines.push(`   Games: ${state.games.size} | Interactions: ${state.stats.interactionsRecorded}`);
  lines.push(`   Equilibria found: ${state.stats.equilibriaFound}`);
  lines.push(`   Focal points: ${state.stats.focalPointsIdentified}`);
  lines.push(`   Cooperation rate: ${(state.stats.cooperationRate * 100).toFixed(1)}%`);
  lines.push('');

  // Recent interactions
  if (state.interactions.length > 0) {
    lines.push('   Recent:');
    const recent = state.interactions.slice(-3);
    for (const int of recent) {
      const game = state.games.get(int.gameId);
      const gameName = game ? game.name : int.gameId;
      lines.push(`   └─ ${gameName}: ${int.strategies.join(' vs ')}`);
    }
  }

  lines.push('');
  return lines.join('\n');
}

function getStats() {
  return {
    ...state.stats,
    gamesCount: state.games.size,
    strategiesRegistered: state.strategies.size,
    repeatedGamesCount: state.repeatedGames.size,
    recentInteractions: state.interactions.slice(-10)
  };
}

// ─────────────────────────────────────────────────────────────
// INITIALIZATION
// ─────────────────────────────────────────────────────────────

function init() {
  loadState();

  // Auto-save periodically
  setInterval(() => saveState(), 60000);

  return {
    initialized: true,
    games: state.games.size,
    strategies: state.strategies.size
  };
}

// ─────────────────────────────────────────────────────────────
// EXPORTS
// ─────────────────────────────────────────────────────────────

module.exports = {
  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,

  // Type definitions
  GAME_TYPES,
  CLASSIC_GAMES,
  EQUILIBRIUM_TYPES,
  COOPERATION_STRATEGIES,
  FOCAL_POINT_TYPES,

  // Core functions
  defineGame,
  getPayoff,
  findNashEquilibria,
  findDominantStrategies,
  findParetoOptimal,
  recordInteraction,
  registerStrategy,
  getStrategyMove,
  identifyFocalPoint,
  analyzeDilemma,
  simulateRepeatedGame,
  analyzeStrategicSituation,

  // State access
  getGame: (id) => state.games.get(id),
  getInteractions: () => [...state.interactions],
  getStrategy: (id) => state.strategies.get(id),

  // Persistence
  saveState,
  loadState,

  // Formatting
  formatStatus,
  getStats,
  init
};
