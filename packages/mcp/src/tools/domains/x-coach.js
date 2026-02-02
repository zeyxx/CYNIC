/**
 * X Communication Coach
 *
 * CYNIC tools to help humans improve their X/Twitter communication.
 * Not to replace them, but to AUTONOMIZE them.
 *
 * "Le chien ne poste pas pour toi. Il t'apprend à mieux communiquer." - κυνικός
 *
 * Tools:
 * - brain_x_coach: Analyze your draft tweet and get suggestions
 * - brain_x_learn: Learn from high-performing tweets
 * - brain_x_style: Analyze and develop your unique voice
 * - brain_x_timing: Best times to post based on your audience
 *
 * @module @cynic/mcp/tools/domains/x-coach
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('XCoach');

// φ constants for scoring
const MAX_CONFIDENCE = PHI_INV; // 0.618 - never claim certainty

// Communication principles (φ-aligned)
const PRINCIPLES = {
  CLARITY: 'Clear message beats clever wordplay',
  AUTHENTICITY: 'Your voice, amplified - not replaced',
  VALUE: 'Give more than you take',
  ENGAGEMENT: 'Invite conversation, not just likes',
  BREVITY: 'Say more with less (φ = less is more)',
};

// Tweet quality dimensions
const QUALITY_DIMENSIONS = [
  { id: 'clarity', name: 'Clarté', weight: 0.20, description: 'Message clair et compréhensible' },
  { id: 'hook', name: 'Accroche', weight: 0.15, description: 'Première ligne captivante' },
  { id: 'value', name: 'Valeur', weight: 0.20, description: 'Apporte quelque chose au lecteur' },
  { id: 'authenticity', name: 'Authenticité', weight: 0.15, description: 'Voix unique et personnelle' },
  { id: 'engagement', name: 'Engagement', weight: 0.15, description: 'Invite à l\'interaction' },
  { id: 'structure', name: 'Structure', weight: 0.10, description: 'Formatage lisible' },
  { id: 'timing', name: 'Timing', weight: 0.05, description: 'Pertinence temporelle' },
];

/**
 * Create X Coach tool - analyze and improve drafts
 */
export function createXCoachTool(judge, localXStore) {
  return {
    name: 'brain_x_coach',
    description: `Analyze your draft tweet and get improvement suggestions.
NOT a ghostwriter - a coach that helps YOU become better.
Provides: score, suggestions, alternative hooks, engagement predictions.`,
    inputSchema: {
      type: 'object',
      properties: {
        draft: {
          type: 'string',
          description: 'Your draft tweet text',
        },
        context: {
          type: 'string',
          description: 'Optional context (replying to someone, topic, goal)',
        },
        style: {
          type: 'string',
          enum: ['informative', 'provocative', 'humorous', 'personal', 'technical'],
          description: 'Intended style/tone',
        },
        goal: {
          type: 'string',
          enum: ['engagement', 'followers', 'conversation', 'authority', 'fun'],
          description: 'What you want to achieve',
        },
      },
      required: ['draft'],
    },
    handler: async (params) => {
      const { draft, context, style = 'informative', goal = 'engagement' } = params;

      if (!draft || draft.trim().length === 0) {
        return { success: false, error: 'Draft text required' };
      }

      try {
        // Analyze the draft
        const analysis = analyzeDraft(draft, { context, style, goal });

        // Get suggestions
        const suggestions = generateSuggestions(draft, analysis, { style, goal });

        // Generate alternative hooks
        const hooks = generateAlternativeHooks(draft, { style, goal });

        // Predict engagement (very rough, φ-capped)
        const prediction = predictEngagement(analysis, goal);

        return {
          success: true,
          analysis: {
            score: Math.min(analysis.score, 100 * MAX_CONFIDENCE), // φ-capped
            dimensions: analysis.dimensions,
            strengths: analysis.strengths,
            weaknesses: analysis.weaknesses,
          },
          suggestions: {
            improvements: suggestions.improvements,
            warnings: suggestions.warnings,
            hooks: hooks,
          },
          prediction: {
            engagement: prediction.level,
            confidence: Math.min(prediction.confidence, MAX_CONFIDENCE),
            note: '*sniff* Les prédictions sont incertaines. φ distrusts φ.',
          },
          principles: {
            applied: suggestions.principlesUsed,
            tip: getRandomPrinciple(),
          },
          meta: {
            charCount: draft.length,
            wordCount: draft.split(/\s+/).length,
            hasEmoji: /\p{Emoji}/u.test(draft),
            hasHashtag: /#\w+/.test(draft),
            hasMention: /@\w+/.test(draft),
            hasLink: /https?:\/\//.test(draft),
          },
        };

      } catch (err) {
        log.error('X Coach error', { error: err.message });
        return {
          success: false,
          error: err.message,
        };
      }
    },
  };
}

/**
 * Create X Learn tool - learn from successful tweets
 */
export function createXLearnTool(localXStore) {
  return {
    name: 'brain_x_learn',
    description: `Learn from high-performing tweets in your captured feed.
Analyzes patterns in viral/engaging content to help you understand what works.`,
    inputSchema: {
      type: 'object',
      properties: {
        topic: {
          type: 'string',
          description: 'Topic to analyze (optional)',
        },
        username: {
          type: 'string',
          description: 'Learn from specific user\'s style',
        },
        minEngagement: {
          type: 'number',
          description: 'Minimum likes+retweets to consider "successful"',
        },
        limit: {
          type: 'number',
          description: 'Number of tweets to analyze (default: 20)',
        },
      },
    },
    handler: async (params) => {
      const { topic, username, minEngagement = 100, limit = 20 } = params;

      if (!localXStore) {
        return {
          success: false,
          error: 'Local X store not available. Capture some tweets first!',
          hint: 'Browse x.com with proxy enabled to capture your feed.',
        };
      }

      try {
        // Get high-performing tweets
        let tweets = [];

        if (username) {
          const user = localXStore.findUserByUsername(username);
          if (user) {
            tweets = localXStore.getTweetsByUser(user.x_user_id, { limit: limit * 2 });
          }
        } else if (topic) {
          tweets = localXStore.searchTweets(topic, { limit: limit * 2 });
        } else {
          tweets = localXStore.getRecentTweets({ limit: limit * 2 });
        }

        // Filter by engagement
        const successful = tweets
          .filter(t => (t.like_count || 0) + (t.retweet_count || 0) >= minEngagement)
          .slice(0, limit);

        if (successful.length === 0) {
          return {
            success: true,
            message: '*head tilt* Pas assez de tweets capturés avec cet engagement.',
            hint: 'Baisse minEngagement ou capture plus de tweets.',
            captured: tweets.length,
          };
        }

        // Analyze patterns
        const patterns = analyzeSuccessPatterns(successful);

        return {
          success: true,
          analyzed: successful.length,
          patterns: {
            hooks: patterns.commonHooks,
            structures: patterns.structures,
            lengths: patterns.lengthStats,
            timing: patterns.timing,
            elements: patterns.elements,
          },
          examples: successful.slice(0, 5).map(t => ({
            text: t.text?.slice(0, 200),
            author: t.username,
            engagement: (t.like_count || 0) + (t.retweet_count || 0),
            why: explainSuccess(t),
          })),
          lessons: patterns.lessons,
          note: '*ears perk* These patterns worked for others. Adapt them to YOUR voice.',
        };

      } catch (err) {
        log.error('X Learn error', { error: err.message });
        return { success: false, error: err.message };
      }
    },
  };
}

/**
 * Create X Style tool - analyze and develop unique voice
 */
export function createXStyleTool(localXStore) {
  return {
    name: 'brain_x_style',
    description: `Analyze your unique communication style based on your tweets.
Helps you understand and develop YOUR voice, not copy others.`,
    inputSchema: {
      type: 'object',
      properties: {
        compareWith: {
          type: 'string',
          description: 'Username to compare style with (optional)',
        },
      },
    },
    handler: async (params) => {
      const { compareWith } = params;

      if (!localXStore) {
        return {
          success: false,
          error: 'Local X store not available.',
        };
      }

      try {
        // Get user's own tweets (marked as is_my_tweet)
        const myTweets = localXStore.searchTweets('', { limit: 100 })
          .filter(t => t.is_my_tweet);

        if (myTweets.length < 5) {
          return {
            success: true,
            message: '*sniff* Need more of YOUR tweets to analyze style.',
            hint: 'Browse your own profile with proxy enabled, or mark tweets as yours.',
            found: myTweets.length,
          };
        }

        // Analyze style
        const style = analyzeStyle(myTweets);

        let comparison = null;
        if (compareWith) {
          const user = localXStore.findUserByUsername(compareWith);
          if (user) {
            const theirTweets = localXStore.getTweetsByUser(user.x_user_id, { limit: 50 });
            if (theirTweets.length > 5) {
              comparison = {
                username: compareWith,
                style: analyzeStyle(theirTweets),
                differences: compareStyles(style, analyzeStyle(theirTweets)),
              };
            }
          }
        }

        return {
          success: true,
          yourStyle: {
            voice: style.voice,
            tone: style.tone,
            vocabulary: style.vocabulary,
            patterns: style.patterns,
            strengths: style.strengths,
            growth: style.growthAreas,
          },
          comparison,
          recommendations: generateStyleRecommendations(style),
          mantra: '*tail wag* Your voice is your moat. Amplify it, don\'t abandon it.',
        };

      } catch (err) {
        log.error('X Style error', { error: err.message });
        return { success: false, error: err.message };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// ANALYSIS HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Analyze a draft tweet
 */
function analyzeDraft(draft, options = {}) {
  const { context, style, goal } = options;

  const dimensions = {};
  let totalScore = 0;
  let totalWeight = 0;

  for (const dim of QUALITY_DIMENSIONS) {
    const score = scoreDimension(draft, dim.id, { context, style, goal });
    dimensions[dim.id] = {
      score,
      weight: dim.weight,
      name: dim.name,
    };
    totalScore += score * dim.weight;
    totalWeight += dim.weight;
  }

  const finalScore = Math.round(totalScore / totalWeight);

  // Identify strengths and weaknesses
  const sorted = Object.entries(dimensions).sort((a, b) => b[1].score - a[1].score);
  const strengths = sorted.slice(0, 2).map(([id, d]) => ({ id, ...d }));
  const weaknesses = sorted.slice(-2).map(([id, d]) => ({ id, ...d }));

  return {
    score: finalScore,
    dimensions,
    strengths,
    weaknesses,
  };
}

/**
 * Score a specific dimension
 */
function scoreDimension(draft, dimensionId, options = {}) {
  const text = draft.toLowerCase();
  const words = draft.split(/\s+/);
  const firstLine = draft.split('\n')[0];

  switch (dimensionId) {
    case 'clarity': {
      // Shorter sentences, common words = higher clarity
      const avgWordLen = words.reduce((a, w) => a + w.length, 0) / words.length;
      const hasJargon = /synergy|leverage|paradigm|disrupt|ecosystem/i.test(draft);
      let score = 70;
      if (avgWordLen < 5) score += 15;
      if (avgWordLen > 7) score -= 20;
      if (hasJargon) score -= 15;
      if (draft.length < 140) score += 10;
      return Math.max(0, Math.min(100, score));
    }

    case 'hook': {
      // First line analysis
      let score = 50;
      if (firstLine.endsWith('?')) score += 20; // Question hook
      if (/^[A-Z]/.test(firstLine) && firstLine.length < 50) score += 10; // Strong start
      if (/hot take|unpopular opinion|thread|🧵/i.test(firstLine)) score += 15;
      if (firstLine.length > 100) score -= 20; // Too long for hook
      if (/^\s*@/.test(firstLine)) score -= 10; // Starts with mention
      return Math.max(0, Math.min(100, score));
    }

    case 'value': {
      let score = 40;
      // Check for actionable content
      if (/how to|tip:|learn|here's|secret|mistake/i.test(draft)) score += 25;
      // Check for unique insight
      if (/I realized|I learned|most people|few know/i.test(draft)) score += 20;
      // Pure opinion without substance
      if (draft.length < 50 && !/\?/.test(draft)) score -= 15;
      return Math.max(0, Math.min(100, score));
    }

    case 'authenticity': {
      let score = 60;
      // Personal pronouns suggest authenticity
      if (/\bI\b|\bmy\b|\bme\b/i.test(draft)) score += 15;
      // Corporate speak reduces authenticity
      if (/excited to announce|proud to|thrilled/i.test(draft)) score -= 20;
      // Emoji can feel authentic if not overdone
      const emojiCount = (draft.match(/\p{Emoji}/gu) || []).length;
      if (emojiCount === 1 || emojiCount === 2) score += 10;
      if (emojiCount > 4) score -= 15;
      return Math.max(0, Math.min(100, score));
    }

    case 'engagement': {
      let score = 40;
      // Questions invite responses
      if (/\?/.test(draft)) score += 25;
      // Call to action
      if (/comment|reply|what do you think|agree\?|thoughts\?/i.test(draft)) score += 20;
      // Controversial but not toxic
      if (/unpopular|controversial|hot take/i.test(draft)) score += 15;
      return Math.max(0, Math.min(100, score));
    }

    case 'structure': {
      let score = 60;
      // Line breaks improve readability
      const lines = draft.split('\n').filter(l => l.trim());
      if (lines.length > 1 && lines.length < 6) score += 20;
      // Bullet points or numbers
      if (/^[\-•]\s|^\d\.\s/m.test(draft)) score += 15;
      // Wall of text penalty
      if (draft.length > 200 && lines.length === 1) score -= 25;
      return Math.max(0, Math.min(100, score));
    }

    case 'timing': {
      // Hard to assess without context, default to neutral
      return 50;
    }

    default:
      return 50;
  }
}

/**
 * Generate improvement suggestions
 */
function generateSuggestions(draft, analysis, options = {}) {
  const { style, goal } = options;
  const improvements = [];
  const warnings = [];
  const principlesUsed = [];

  // Based on weak dimensions
  for (const weak of analysis.weaknesses) {
    if (weak.score < 50) {
      switch (weak.id) {
        case 'clarity':
          improvements.push('Simplifie: remplace les mots complexes par des mots simples.');
          principlesUsed.push(PRINCIPLES.CLARITY);
          break;
        case 'hook':
          improvements.push('Accroche faible. Commence par une question ou une affirmation forte.');
          break;
        case 'value':
          improvements.push('Ajoute de la valeur: un conseil, un insight, une info utile.');
          principlesUsed.push(PRINCIPLES.VALUE);
          break;
        case 'engagement':
          improvements.push('Termine par une question pour inviter les réponses.');
          principlesUsed.push(PRINCIPLES.ENGAGEMENT);
          break;
        case 'structure':
          improvements.push('Ajoute des sauts de ligne pour aérer le texte.');
          break;
      }
    }
  }

  // Length warnings
  if (draft.length > 250) {
    warnings.push('Tweet long. Considère un thread ou coupe le superflu.');
    principlesUsed.push(PRINCIPLES.BREVITY);
  }

  // Hashtag warnings
  const hashtagCount = (draft.match(/#\w+/g) || []).length;
  if (hashtagCount > 2) {
    warnings.push('Trop de hashtags. 1-2 max pour ne pas paraître spam.');
  }

  // Link warning
  if (/https?:\/\//.test(draft)) {
    warnings.push('Les tweets avec liens ont souvent moins de reach. Mets le lien en reply?');
  }

  return { improvements, warnings, principlesUsed };
}

/**
 * Generate alternative hooks
 */
function generateAlternativeHooks(draft, options = {}) {
  const { style, goal } = options;
  const hooks = [];
  const firstLine = draft.split('\n')[0];
  const content = draft.replace(firstLine, '').trim();

  // Question hook
  hooks.push({
    type: 'question',
    text: `What if ${firstLine.toLowerCase().replace(/[.!]$/, '')}?`,
    why: 'Les questions invitent à la réflexion',
  });

  // Bold statement hook
  hooks.push({
    type: 'bold',
    text: `Hot take: ${firstLine}`,
    why: 'Polarise et attire l\'attention',
  });

  // Personal hook
  hooks.push({
    type: 'personal',
    text: `I used to think... but now I realize ${firstLine.toLowerCase()}`,
    why: 'Le parcours personnel crée la connexion',
  });

  return hooks.slice(0, 3);
}

/**
 * Predict engagement (very rough)
 */
function predictEngagement(analysis, goal) {
  const score = analysis.score;

  let level, confidence;

  if (score > 75) {
    level = 'high';
    confidence = 0.4; // Still uncertain
  } else if (score > 50) {
    level = 'medium';
    confidence = 0.5;
  } else {
    level = 'low';
    confidence = 0.55; // More confident about low quality
  }

  return {
    level,
    confidence: Math.min(confidence, MAX_CONFIDENCE),
  };
}

/**
 * Analyze success patterns in high-performing tweets
 */
function analyzeSuccessPatterns(tweets) {
  const hooks = {};
  const structures = { singleLine: 0, multiLine: 0, thread: 0 };
  const lengths = [];
  const elements = { questions: 0, emoji: 0, links: 0, mentions: 0 };

  for (const t of tweets) {
    const text = t.text || '';
    lengths.push(text.length);

    // Hook patterns
    const firstWord = text.split(/\s/)[0]?.toLowerCase();
    hooks[firstWord] = (hooks[firstWord] || 0) + 1;

    // Structure
    const lines = text.split('\n').length;
    if (lines === 1) structures.singleLine++;
    else if (lines < 5) structures.multiLine++;
    else structures.thread++;

    // Elements
    if (/\?/.test(text)) elements.questions++;
    if (/\p{Emoji}/u.test(text)) elements.emoji++;
    if (/https?:\/\//.test(text)) elements.links++;
    if (/@\w+/.test(text)) elements.mentions++;
  }

  // Sort hooks by frequency
  const commonHooks = Object.entries(hooks)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([word, count]) => ({ word, count, pct: Math.round(count / tweets.length * 100) }));

  // Length stats
  const avgLength = Math.round(lengths.reduce((a, b) => a + b, 0) / lengths.length);

  return {
    commonHooks,
    structures,
    lengthStats: {
      average: avgLength,
      min: Math.min(...lengths),
      max: Math.max(...lengths),
    },
    elements: {
      questionsRate: Math.round(elements.questions / tweets.length * 100),
      emojiRate: Math.round(elements.emoji / tweets.length * 100),
      linksRate: Math.round(elements.links / tweets.length * 100),
    },
    timing: null, // Would need timestamp analysis
    lessons: [
      commonHooks[0] ? `Les tweets commençant par "${commonHooks[0].word}" performent bien` : null,
      elements.questions > tweets.length * 0.3 ? 'Les questions génèrent de l\'engagement' : null,
      avgLength < 150 ? 'La concision paie' : null,
    ].filter(Boolean),
  };
}

/**
 * Explain why a tweet was successful
 */
function explainSuccess(tweet) {
  const reasons = [];
  const text = tweet.text || '';

  if (/\?/.test(text)) reasons.push('Question engageante');
  if (text.length < 100) reasons.push('Concis et percutant');
  if (/hot take|unpopular/i.test(text)) reasons.push('Prise de position forte');
  if (/I |my /i.test(text)) reasons.push('Authenticité personnelle');

  return reasons.length > 0 ? reasons.join(', ') : 'Contenu de qualité';
}

/**
 * Analyze communication style from tweets
 */
function analyzeStyle(tweets) {
  const allText = tweets.map(t => t.text || '').join(' ');
  const words = allText.toLowerCase().split(/\s+/);

  // Tone analysis (simplified)
  const positiveWords = words.filter(w => /good|great|love|amazing|excited|happy/i.test(w)).length;
  const negativeWords = words.filter(w => /bad|hate|terrible|angry|sad|worst/i.test(w)).length;
  const technicalWords = words.filter(w => /code|api|data|system|build|ship/i.test(w)).length;

  const totalWords = words.length || 1;

  return {
    voice: technicalWords / totalWords > 0.05 ? 'Technical' : 'Conversational',
    tone: positiveWords > negativeWords ? 'Positive' : 'Neutral/Critical',
    vocabulary: {
      avgWordLength: Math.round(words.reduce((a, w) => a + w.length, 0) / totalWords * 10) / 10,
      technicalRatio: Math.round(technicalWords / totalWords * 100),
    },
    patterns: {
      usesQuestions: tweets.filter(t => /\?/.test(t.text)).length / tweets.length > 0.2,
      usesEmoji: tweets.filter(t => /\p{Emoji}/u.test(t.text)).length / tweets.length > 0.3,
      usesThreads: tweets.filter(t => (t.text?.split('\n').length || 0) > 3).length / tweets.length > 0.1,
    },
    strengths: [],
    growthAreas: [],
  };
}

/**
 * Compare two styles
 */
function compareStyles(style1, style2) {
  return {
    voiceDiff: style1.voice !== style2.voice,
    toneDiff: style1.tone !== style2.tone,
    note: 'Style comparison helps understand differences, not copy.',
  };
}

/**
 * Generate style recommendations
 */
function generateStyleRecommendations(style) {
  const recs = [];

  if (!style.patterns.usesQuestions) {
    recs.push('Pose plus de questions pour engager ton audience');
  }

  if (style.vocabulary.avgWordLength > 6) {
    recs.push('Simplifie ton vocabulaire pour plus de clarté');
  }

  if (style.tone === 'Neutral/Critical') {
    recs.push('Un peu plus de positivité peut attirer plus de followers');
  }

  return recs;
}

/**
 * Get a random communication principle
 */
function getRandomPrinciple() {
  const keys = Object.keys(PRINCIPLES);
  const key = keys[Math.floor(Math.random() * keys.length)];
  return PRINCIPLES[key];
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

export const xCoachFactory = {
  name: 'x-coach',
  domain: 'social',
  requires: [],

  create(options) {
    const { localXStore, judge } = options;

    return [
      createXCoachTool(judge, localXStore),
      createXLearnTool(localXStore),
      createXStyleTool(localXStore),
    ];
  },
};

export default xCoachFactory;
