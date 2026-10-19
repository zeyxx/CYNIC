# Temporal anchoring + human/LLM-agent alignment (chantier)

## Status

Declared 2026-10-18 by the operator after the agent produced ungrounded
time claims ("ce soir", "demain", "il est tard") across a 3-day session
without ever calling `date`. This document is the first artifact of the
correction loop it describes.

## Problem

LLM agents confabulate temporal context (time-of-day, elapsed durations,
"yesterday") from priors instead of observing it. Every ungrounded marker
corrupts receipts, sequencing decisions ("wait until tomorrow"), and the
operator's trust in agent-reported timelines.

## Mechanism

1. **Observed time only:** no agent states clock time, dates, durations,
   or day-boundaries except from a tool call (`date`, file mtimes read
   from disk, API timestamps). Model priors about "now" are not a source.
2. **Receipts carry observed timestamps:** every proof receipt records
   when it was measured (tool output), never when the agent narrates it.
3. **Session context "today" is SSOT:** the injected current date
   overrides any derived sense of time; on conflict, re-query, never
   interpolate.
4. **Correction loop:** operator temporal correction → agent re-anchors
   immediately, records the miss in the tranche log, adjusts the pending
   claim. Silence is not consent; a wrong guess is not a placeholder.

## Non-goals

- No NTP/daemon work (OS concern, out of agent scope).
- No retroactive rewrite of past receipts (append corrections, never edit).
