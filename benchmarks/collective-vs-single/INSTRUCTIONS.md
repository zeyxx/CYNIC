# Benchmark: Collective vs Single

## Objectif
Comparer les reviews de code entre:
- **Vanilla Claude**: Claude sans contexte CYNIC
- **CYNIC Collective**: Claude avec orchestration CYNIC

## Étape 1: Vanilla Claude (toi)

Ouvre un nouveau terminal HORS de ce repo:
```bash
cd /tmp
claude
```

Pour chaque fichier, donne ce prompt exact à Claude:

### Prompt (copie-colle exactement):
```
Review this code for:
1. Security vulnerabilities
2. Logic errors / bugs
3. Performance issues
4. Code quality problems

Be specific. List each issue with line number if possible.
Format: one issue per line, severity in brackets [CRITICAL/HIGH/MEDIUM/LOW].

Code:
```

Puis colle le contenu du fichier.

### Fichiers à review:
1. `code-samples/sample1.js` → sauvegarde output dans `outputs-vanilla/sample1.txt`
2. `code-samples/sample2.js` → sauvegarde output dans `outputs-vanilla/sample2.txt`
3. `code-samples/sample3.js` → sauvegarde output dans `outputs-vanilla/sample3.txt`

## Étape 2: CYNIC Collective (moi)

Je vais run les mêmes fichiers avec CYNIC et sauvegarder dans `outputs-cynic/`.

## Étape 3: Comparaison

Je compare les outputs:
- Nombre d'issues trouvées
- Types d'issues (security vs logic vs quality)
- Overlap / différences

## Fichiers

```
benchmarks/collective-vs-single/
├── code-samples/
│   ├── sample1.js (postgres client)
│   ├── sample2.js (guard hook)
│   └── sample3.js (judgment tools)
├── outputs-vanilla/
│   ├── sample1.txt (à remplir par toi)
│   ├── sample2.txt
│   └── sample3.txt
├── outputs-cynic/
│   ├── sample1.txt (généré par moi)
│   ├── sample2.txt
│   └── sample3.txt
└── INSTRUCTIONS.md (ce fichier)
```
