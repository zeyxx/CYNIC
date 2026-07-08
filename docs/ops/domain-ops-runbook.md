# Domain Ops Runbook — talaria.build

**Scope** : opérations publiques sur `talaria.build` et ses sous-domaines.  
**Organe** : `organ-domain` (`organs/domain/`)  
**Desired state** : `organs/domain/desired/domains.json`

---

## Ordre de lecture de l'agent

1. Ce runbook
2. `organs/domain/desired/domains.json` (desired state courant)
3. La commande d'organe correspondante

L'agent ne doit pas "se souvenir" de comment DNS fonctionnait la dernière fois. Il lit le desired state et exécute les étapes gardées à chaque fois.

---

## Prérequis

```bash
# Toujours sourcer les credentials avant toute commande
source ~/.cynic-env

# Vérifier que les variables requises sont présentes
echo $PORKBUN_API_KEY && echo $CF_API_TOKEN && echo $CF_ACCOUNT_ID
```

---

## Workflow A — Migration de zone (une seule fois)

**Contexte** : `talaria.build` est chez Porkbun (registrar + DNS). L'objectif est de garder Porkbun comme registrar et de transférer l'autorité DNS à Cloudflare.

### Étape 1 — Audit du portfolio et de l'expiry
```bash
python3 organs/domain/src/entrypoints/cli.py registrar portfolio
python3 organs/domain/src/entrypoints/cli.py registrar expiry-audit
```
Non-mutating. Confirmer que `talaria.build` est bien dans le portfolio.

### Étape 2 — Backup de la zone actuelle (obligatoire avant toute écriture)
```bash
python3 organs/domain/src/entrypoints/cli.py zone backup
```
Sauvegarde dans `organs/domain/state/backups/`. À vérifier avant de continuer.

### Étape 3 — Dry-run de la migration
```bash
python3 organs/domain/src/entrypoints/cli.py zone migrate --dry-run
```
Inspecter la liste des records qui seraient créés dans Cloudflare. Confirmer qu'aucun record critique n'est absent.

### Étape 4 — Migration effective (gate : approbation humaine requise)
```bash
python3 organs/domain/src/entrypoints/cli.py zone migrate
```
Crée la zone Cloudflare et importe les records. Types importés : A, AAAA, CNAME, MX, TXT. NS registrar ignorés.

### Étape 5 — Récupérer le zone_id Cloudflare
```bash
# Le zone migrate affiche le zone_id dans sa sortie JSON
# Conserver le zone_id pour les étapes suivantes
```

### Étape 6 — Dry-run du NS cutover (gate : approbation humaine requise)
```bash
python3 organs/domain/src/entrypoints/cli.py zone set-ns \
  --apex talaria.build \
  --zone-id <zone_id> \
  --dry-run
```
Confirmer les nameservers Cloudflare qui seront configurés.

### Étape 7 — NS cutover effectif (gate : approbation humaine requise, irréversible à court terme)
```bash
python3 organs/domain/src/entrypoints/cli.py zone set-ns \
  --apex talaria.build \
  --zone-id <zone_id>
```

### Étape 8 — Vérification de la délégation TLD
```bash
python3 organs/domain/src/entrypoints/cli.py zone verify-delegation \
  --apex talaria.build \
  --zone-id <zone_id>
```
Vérifie via les NS authoritatifs TLD — pas de faux négatifs liés à la propagation globale.

---

## Workflow B — Déploiement d'une app (répétable)

**Contexte** : zone Cloudflare active. L'app est déclarée dans `desired/domains.json`.

### Étape 1 — Dry-run du déploiement
```bash
python3 organs/domain/src/entrypoints/cli.py pages deploy \
  --app odysseum \
  --dry-run
```

### Étape 2 — Déploiement effectif (gate : approbation humaine requise)
```bash
python3 organs/domain/src/entrypoints/cli.py pages deploy \
  --app odysseum
```
Séquence interne :
1. CF Pages project créé ou localisé
2. `wrangler pages deploy`
3. `odysseum.talaria.build` attaché comme custom domain
4. Probe HTTP apex

### Étape 3 — Vérification HTTP
```bash
python3 organs/domain/src/entrypoints/cli.py pages verify
```
Probe toutes les apps déclarées dans desired state. Résout via NS authoritatifs Cloudflare.

---

## Ajouter une nouvelle app

1. Ajouter une entrée dans `organs/domain/desired/domains.json` :
```json
{
  "subdomain": "mon-app",
  "fqdn": "mon-app.talaria.build",
  "pages_project": "mon-app",
  "source_dir": "apps/mon-app/dist",
  "analytics": true,
  "status": "pending"
}
```
2. Exécuter Workflow B avec `--app mon-app`

---

## Gates

| Opération | Gate |
|---|---|
| Achat de domaine | Confirmation humaine avec montant explicite |
| Zone migrate | Confirmation humaine (vérifier dry-run d'abord) |
| NS cutover | Confirmation humaine séparée (irréversible à court terme) |
| Pages deploy | Confirmation humaine |
| DNS record write | Confirmation humaine |

Les opérations read-only (portfolio, expiry, backup, dry-run, verify) n'ont pas de gate.

---

## Frontières

- `organ-domain` gère : DNS public `talaria.build`, CF Pages, redirects Cloudflare
- `organ-wireguard` gère : DNS interne Synapse (mesh privé)
- `organ-headscale` gère : accès mesh opérateur/agent

Ne pas confondre les deux surfaces DNS.
