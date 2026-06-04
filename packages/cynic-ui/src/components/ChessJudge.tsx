import { useState, useRef, useCallback, useMemo } from 'react';
import { Chess } from 'chess.js';
import type { Square } from 'chess.js';
import { judge } from '../api';
import type { Verdict } from '../types';
import { VerdictDisplay } from './VerdictDisplay';
import { getSelectedDogs } from '../utils';

// ─── Constants ────────────────────────────────────────────────
const LIGHT = '#b0bec5';
const DARK = '#37474f';
const HIGHLIGHT_MOVE = 'rgba(201,168,76,0.45)';
const HIGHLIGHT_SELECTED = 'rgba(201,168,76,0.75)';
const HIGHLIGHT_CHECK = 'rgba(244,67,54,0.55)';
const HIGHLIGHT_LAST = 'rgba(100,181,246,0.4)';

type PromotionPiece = 'q' | 'r' | 'b' | 'n';

interface Props {
  onVerdict?: (verdict: Verdict) => void;
}

// ─── Status helper (pure, no hooks) ───────────────────────────
function gameStatus(g: Chess) {
  if (g.isCheckmate()) {
    const w = g.turn() === 'w' ? 'Black' : 'White';
    return { label: `♛ CHECKMATE — ${w} wins`, color: '#F44336', over: true };
  }
  if (g.isStalemate())           return { label: '⚡ STALEMATE — Draw',              color: '#FF9800', over: true };
  if (g.isInsufficientMaterial())return { label: '🔔 INSUFFICIENT MATERIAL — Draw',  color: '#FF9800', over: true };
  if (g.isThreefoldRepetition()) return { label: '🔄 THREEFOLD REPETITION — Draw',   color: '#FF9800', over: true };
  if (g.isDraw())                return { label: '½-½ DRAW',                          color: '#FF9800', over: true };
  if (g.isCheck()) {
    const side = g.turn() === 'w' ? 'White' : 'Black';
    return { label: `⚠ CHECK — ${side} in check`, color: '#FF9800', over: false };
  }
  const side = g.turn() === 'w' ? 'White' : 'Black';
  return { label: `${side} to move`, color: g.turn() === 'w' ? '#e0e0e0' : '#aaa', over: false };
}

function findKing(g: Chess, color: 'w' | 'b'): Square | null {
  const files = ['a','b','c','d','e','f','g','h'];
  const ranks = ['1','2','3','4','5','6','7','8'];
  for (const f of files) for (const r of ranks) {
    const sq = (f + r) as Square;
    const p = g.get(sq);
    if (p && p.type === 'k' && p.color === color) return sq;
  }
  return null;
}

export function ChessJudge({ onVerdict }: Props) {
  return <div style={{ color: '#C9A84C', padding: 20 }}>Chessboard component disabled for debugging.</div>;
  /*
  // The game object lives in a ref — mutation is direct, no stale closures
  const gameRef = useRef(new Chess());
  // ... rest of the original code ...
  */
}
