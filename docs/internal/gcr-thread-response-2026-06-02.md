# Réponse thread GCR — 2026-06-02

## Contexte

GCR a proposé : chess PoH → SOL raffle quotidien (pool Robinhood fees, >0.1 SOL winner takes all).
Ragnar a accepté mais suggère d'ajouter des questions math/culture en plus du chess.
GCR dit ok pour commencer avec un captcha simple puis porter vers chess.

## Message à envoyer dans le thread (T. → thread)

```
Love the raffle idea — that's the hook.

Let's scope it together:

Phase 1 (this week): simple verification gate, fast to ship
- Short chess game OR quick math puzzle (S.'s idea — adds robustness without delaying launch)
- Verified = enter raffle
- GCR takes cut per verification

Phase 2 (when chess is tuned): pure chess gate
- 5 min game, Stockfish validates human behavior
- Port the same raffle mechanic

For phase 1, Ragnar — what do you need from CultScreener's side to wire it? Just a callback URL when verification is complete?
```

## Ce qu'il faut clarifier avec S. (message séparé)

- Le endpoint B&C à exposer : POST `/api/verify` → { wallet, verified: bool }
- Comment CultScreener appelle ce endpoint (webhook ou polling)
- Le cut GCR : % par badge mint ou flat fee par raffle entry

## Notes

- GCR a dit "no problem if it takes longer" → pas de pression mort si phase 1 prend 2-3 jours
- Le writeup CYNIC × CultScreener est séparé de ce sujet (ne pas mélanger les deux pitches dans ce thread)
- La landing talaria.build dit déjà "Mobile-first · tournaments · tribes" — aligne avec ce que S. veut builder
