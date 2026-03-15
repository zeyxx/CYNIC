# Sovereign Stack Alignment — Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement this plan.

**Goal:** Aligner SurrealDB, Python, les clients, et le pipeline CI/CD en un stack cohérent et souverain.

**Architecture:**
- Forge = source de vérité et CI/CD
- CYNIC kernel = seul point de contact avec SurrealDB
- KAIROS = agent de CYNIC, parle via gRPC (plus de SurrealDB direct)
- Pipeline = générique, détection automatique, `scripts/validate.sh` dans chaque repo

**Tech Stack:** Rust 1.94 · Python 3.13.12 · SurrealDB 3.0.3 · tonic 0.12 · uv · Bash

---

## Ordre d'exécution (révisé après crystallize-truth)

```
Chunk 1  →  Pipeline CI/CD (forge-lib.sh + hook générique + validate.sh)
Chunk 2  →  CYNIC : vrais tests d'intégration SurrealDB (filet de sécurité)
Chunk 3  →  Forge Infra (SurrealDB 3.0.3 + Python 3.13.12 pin)
Chunk 4  →  KAIROS : supprimer SurrealDB direct, ajouter gRPC CYNIC
Chunk 5  →  Maturité architecturale (dette explicite, déclencheurs concrets)
```

**Pourquoi cet ordre :**
- Pipeline d'abord : pas d'upgrade sans validation automatisée
- Vrais tests CYNIC avant upgrade SurrealDB : les tests sont le filet de sécurité
- KAIROS gRPC en dernier : dépend de CYNIC stable

---

## Chunk 1 — Pipeline CI/CD

> Fondation. Rien ne s'exécute sur forge sans ce pipeline.

---

### Task 1 : forge-lib.sh sur forge

**Fichiers touchés :** `~/.forge-lib.sh` (forge, une fois — non versionné)

- [ ] Créer `~/.forge-lib.sh` sur forge
```bash
ssh forge 'cat > ~/.forge-lib.sh' << 'LIB'
#!/bin/bash
forge_log()  { echo "[forge $(date '+%H:%M:%S')] $*"; }

forge_lock() {
    exec 9>/tmp/forge-validate.lock
    flock -n 9 || { forge_log "❌ Validation en cours — retry dans 30s"; exit 1; }
}

forge_detect_bypass() {
    local msg; msg=$(git log -1 --format="%s" HEAD)
    echo "$msg" | grep -qiE 'trigger.*(forge|ci|valid)|ci.*trigger' && {
        forge_log "❌ BYPASS: '$msg' — corrige le code."
        exit 1
    }
}

forge_check_secrets() {
    command -v gitleaks &>/dev/null && \
        gitleaks detect --source=. --no-git -q 2>/dev/null && return 0
    forge_log "⚠️  gitleaks absent — secrets non vérifiés"
}

forge_check_commit_format() {
    local msg; msg=$(git log -1 --format="%s" HEAD)
    echo "$msg" | grep -qE '^(feat|fix|chore|docs|refactor|test|ci|perf|build)\([^)]+\): .{5,}' || {
        forge_log "❌ Format commit invalide: '$msg'"
        forge_log "   Requis: type(scope): description (5+ chars)"
        exit 1
    }
}

forge_push_github() {
    local repo="$1" branch="${2:-main}"
    git remote add github "git@github.com:zeyxx/${repo}.git" 2>/dev/null || true
    git push github "$branch" || forge_log "⚠️  GitHub mirror failed (non-bloquant)"
}

forge_audit() {
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) | $1 | $2 | $3 | ${4}s" >> "$HOME/forge-audit.log"
}
LIB
chmod +x ~/.forge-lib.sh
```

- [ ] Vérifier
```bash
ssh forge "source ~/.forge-lib.sh && forge_log 'lib OK'"
# Attendu : [forge HH:MM:SS] lib OK
```

---

### Task 2 : Hook générique sur forge

**Fichiers touchés :** `~/CYNIC.git/hooks/post-receive` · `~/KAIROS.git/hooks/post-receive` (forge)

- [ ] Déployer le hook générique sur forge
```bash
ssh forge 'cat > /tmp/generic-hook' << 'HOOK'
#!/bin/bash
set -e
source "$HOME/.forge-lib.sh"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

REPO_NAME=$(basename "$(git rev-parse --git-dir)" .git)
WORK_DIR="$HOME/${REPO_NAME}-validate"
START=$(date +%s)

forge_lock
forge_log "Push $REPO_NAME reçu..."

rm -rf "$WORK_DIR"
git clone --branch main "$(git rev-parse --git-dir)" "$WORK_DIR"
unset GIT_DIR GIT_WORK_TREE
cd "$WORK_DIR"

forge_detect_bypass
forge_check_secrets
forge_check_commit_format

if [ -f scripts/validate.sh ]; then
    bash scripts/validate.sh
else
    forge_log "⚠️  scripts/validate.sh absent — skip validation"
fi

COMMIT=$(git rev-parse --short HEAD)
forge_push_github "$REPO_NAME"
forge_audit "$REPO_NAME" "$COMMIT" "OK" "$(( $(date +%s) - START ))"
forge_log "✅ ${REPO_NAME} validé en $(( $(date +%s) - START ))s"
HOOK

ssh forge 'cp /tmp/generic-hook ~/CYNIC.git/hooks/post-receive && \
           cp /tmp/generic-hook ~/KAIROS.git/hooks/post-receive && \
           chmod +x ~/CYNIC.git/hooks/post-receive ~/KAIROS.git/hooks/post-receive'
```

- [ ] Vérifier que les hooks sont exécutables
```bash
ssh forge "ls -la ~/CYNIC.git/hooks/post-receive ~/KAIROS.git/hooks/post-receive"
# Attendu : -rwxr-xr-x
```

---

### Task 3 : scripts/validate.sh dans CYNIC-V2

**Fichiers touchés :** `CYNIC-V2/scripts/validate.sh`

- [ ] Créer `scripts/validate.sh`
```bash
#!/bin/bash
set -e
source "$HOME/.cargo/env"

echo "[cynic] clippy..."
cargo clippy --workspace -- -D warnings

echo "[cynic] test..."
SURREALDB_URL=http://localhost:8000 \
SURREALDB_USER=root \
SURREALDB_PASS=$(cat ~/.surreal-pass) \
cargo test --workspace

echo "[cynic] audit..."
cargo audit || true
```

- [ ] Rendre exécutable et committer
```bash
chmod +x scripts/validate.sh
git add scripts/validate.sh
git commit -m "ci(forge): add scripts/validate.sh — project-owned pipeline"
git push forge main
```

---

### Task 4 : scripts/validate.sh dans KAIROS

**Fichiers touchés :** `KAIROS/scripts/validate.sh`

> KAIROS ne teste PAS SurrealDB directement — il dépendra de CYNIC via gRPC.

- [ ] Créer `scripts/validate.sh` dans KAIROS
```bash
#!/bin/bash
set -e
export PATH="$HOME/.local/bin:$PATH"

echo "[kairos] sync..."
uv sync --extra dev

echo "[kairos] ruff..."
uv run ruff check .

echo "[kairos] mypy..."
uv run mypy . --ignore-missing-imports || true

echo "[kairos] pytest..."
uv run pytest --tb=short -q -m "not live"
```

- [ ] Rendre exécutable et committer
```bash
chmod +x scripts/validate.sh
git add scripts/validate.sh
git commit -m "ci(forge): add scripts/validate.sh — no direct SurrealDB"
git push forge main
```

---

## Chunk 2 — CYNIC : Vrais tests d'intégration

> Filet de sécurité avant tout upgrade. Mocks = hérésie.

---

### Task 5 : Ajouter ~/.surreal-pass sur forge

**Fichiers touchés :** `~/.surreal-pass` (forge, une fois — confidentiel)

- [ ] Créer le fichier de credentials
```bash
ssh forge "echo '<REDACTED>' > ~/.surreal-pass && chmod 600 ~/.surreal-pass"
```

- [ ] Vérifier la connexion SurrealDB actuelle
```bash
ssh forge "surreal sql --conn http://localhost:8000 \
  --user root --pass <REDACTED> \
  --ns test --db test \
  'SELECT * FROM test LIMIT 1' 2>&1"
# Attendu : pas d'erreur de protocole
```

---

### Task 6 : Ajouter init_with() dans CynicStorage

**Fichiers touchés :** `cynic-kernel/src/storage.rs`

> `init()` utilise des env vars. `init_with()` prend les paramètres explicitement — nécessaire pour les tests.

- [ ] Écrire le test en premier (TDD)
```rust
// Dans cynic-kernel/src/storage.rs, section #[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_and_retrieve_fact() {
        let url = std::env::var("SURREALDB_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let storage = CynicStorage::init_with(&url, "test_cynic", "ci").await
            .expect("SurrealDB doit être accessible sur forge");

        // Stocker un fait
        let result = storage.db.query(
            "CREATE fact SET content = $content, confidence = $conf"
        )
        .bind(("content", "test fact from integration test"))
        .bind(("conf", 0.9f64))
        .await;
        assert!(result.is_ok(), "store_fact doit réussir");

        // Le relire
        let mut resp = storage.db.query("SELECT * FROM fact WHERE content = $c")
            .bind(("c", "test fact from integration test"))
            .await
            .expect("SELECT doit réussir");
        let rows: Vec<serde_json::Value> = resp.take(0).expect("take(0)");
        assert!(!rows.is_empty(), "Le fait doit être retrouvé");

        // Nettoyage
        storage.db.query("DELETE fact WHERE content = $c")
            .bind(("c", "test fact from integration test"))
            .await.ok();
    }
}
```

- [ ] Ajouter `init_with()` dans `CynicStorage`
```rust
impl CynicStorage {
    pub async fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let db_url = std::env::var("SURREALDB_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        Self::init_with(&db_url, "cynic", "v2").await
    }

    pub async fn init_with(url: &str, ns: &str, db_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db: Surreal<Any> = Surreal::init();
        db.connect(url).await?;
        db.use_ns(ns).use_db(db_name).await?;
        println!("[Ring 1 / UAL] Linked to Sidecar Memory at {}", url);
        Ok(Self { db })
    }
}
```

- [ ] Lancer les tests sur forge (via git push)
```bash
git add cynic-kernel/src/storage.rs
git commit -m "test(storage): real SurrealDB integration tests — no mocks"
git push forge main
# Attendu : 6/6 tests passent (5 MockBackend + 1 intégration SurrealDB)
```

---

## Chunk 3 — Forge Infra : Versions stables

> Upgrade avec filet de sécurité (Chunk 2 doit être vert avant).

---

### Task 7 : SurrealDB 3.0.3 stable sur forge

**Fichiers touchés :** `/usr/local/bin/surreal` · `/etc/systemd/system/surrealdb.service` (forge)

- [ ] Vérifier la version actuelle
```bash
ssh forge "/usr/local/bin/surreal version"
# Attendu actuel : 3.1.0-nightly
```

- [ ] Télécharger SurrealDB 3.0.3
```bash
ssh forge "curl -L https://github.com/surrealdb/surrealdb/releases/download/v3.0.3/surreal-v3.0.3.linux-amd64.tgz \
  | tar xz -C /tmp && sudo mv /tmp/surreal /usr/local/bin/surreal-3.0.3"
```

- [ ] Remplacer le binaire
```bash
ssh forge "sudo systemctl stop surrealdb && \
  sudo mv /usr/local/bin/surreal /usr/local/bin/surreal-nightly.bak && \
  sudo mv /usr/local/bin/surreal-3.0.3 /usr/local/bin/surreal && \
  sudo systemctl start surrealdb"
```

- [ ] Vérifier
```bash
ssh forge "/usr/local/bin/surreal version && systemctl is-active surrealdb"
# Attendu : surreal-3.0.3 · active
```

- [ ] Re-lancer les tests CYNIC (validation de non-régression)
```bash
ssh forge "cd ~/CYNIC-validate && cargo test --workspace 2>&1"
# Attendu : 6/6 ✅ (même résultat qu'avant l'upgrade)
```

---

### Task 8 : Mettre à jour le crate surrealdb vers 3.0.3

**Fichiers touchés :** `cynic-kernel/Cargo.toml` · `cynic-kernel/src/storage.rs`

- [ ] Mettre à jour la dépendance dans `Cargo.toml`
```toml
# cynic-kernel/Cargo.toml
surrealdb = { version = "3.0.3", features = ["protocol-http"] }
```

- [ ] Vérifier les breaking changes via `cargo check`
```bash
# Localement ou via git push forge
cargo check --workspace 2>&1
# Lister TOUTES les erreurs avant de corriger quoi que ce soit
```

Points connus à vérifier dans `storage.rs` :
- `Surreal::<Any>::init()` → API inchangée ?
- `.connect()` signature
- `.query().bind()` signature
- `.take()` sur les résultats

- [ ] Corriger `storage.rs` selon les erreurs (voir les breaking changes 3.x)

- [ ] Lancer les tests
```bash
cargo test --workspace 2>&1
# Attendu : 6/6 ✅
```

- [ ] Commit
```bash
git add cynic-kernel/Cargo.toml cynic-kernel/src/storage.rs cynic-kernel/Cargo.lock
git commit -m "feat(storage): upgrade surrealdb crate 2.x → 3.0.3"
git push forge main
```

---

### Task 9 : Python 3.13.12 pinné dans KAIROS

**Fichiers touchés :** `pyproject.toml` · `.python-version` (repo KAIROS)

> Python 3.13.12 est déjà installé sur forge via uv. Cette tâche pin la version dans le repo.

- [ ] Mettre à jour `requires-python` dans KAIROS `pyproject.toml`
```toml
[project]
requires-python = ">=3.13.12"
```

- [ ] Créer `.python-version`
```
3.13.12
```

- [ ] Vérifier que uv respecte la contrainte
```bash
cd KAIROS && uv sync --extra dev 2>&1 | head -5
# Attendu : uv utilise cpython-3.13.12
```

- [ ] Commit
```bash
git add pyproject.toml .python-version
git commit -m "chore(python): pin Python 3.13.12"
git push forge main
```

---

## Chunk 4 — KAIROS : Supprimer SurrealDB direct, ajouter gRPC CYNIC

> KAIROS est un agent de CYNIC. Il parle à CYNIC, pas à SurrealDB.
> Prérequis : CYNIC stable (Chunks 1-3 complétés).

---

### Task 10 : Supprimer surrealdb de KAIROS

**Fichiers touchés :** `pyproject.toml` · `ingestion/db.py` · `tests/ingestion/test_db.py`

- [ ] Supprimer la dépendance surrealdb
```toml
# pyproject.toml — retirer
"surrealdb>=1.0.0",
```

- [ ] Ajouter gRPC client
```toml
# pyproject.toml — ajouter dans dependencies
"grpcio>=1.60.0",
"grpcio-tools>=1.60.0",
"protobuf>=5.0.0",
```

- [ ] Supprimer `ingestion/db.py` (remplacé par client gRPC)

- [ ] Commit intermédiaire
```bash
git add pyproject.toml ingestion/db.py
git commit -m "refactor(db): remove direct SurrealDB — replaced by CYNIC gRPC"
```

---

### Task 11 : Client gRPC CognitiveMemory dans KAIROS

**Fichiers touchés :**
- Créer : `kairos/cynic_client.py`
- Créer : `protos/cynic.proto` (copie depuis CYNIC-V2)
- Créer : `kairos/generated/` (code généré)
- Modifier : `ingestion/` — remplacer appels `db.py` par `cynic_client.py`

- [ ] Copier le proto de CYNIC dans KAIROS
```bash
cp CYNIC-V2/protos/cynic.proto KAIROS/protos/
```

- [ ] Générer le code gRPC Python
```bash
cd KAIROS
mkdir -p kairos/generated
uv run python -m grpc_tools.protoc \
  -I protos \
  --python_out=kairos/generated \
  --grpc_python_out=kairos/generated \
  protos/cynic.proto
```

- [ ] Créer `kairos/cynic_client.py`
```python
import os
import grpc
from kairos.generated import cynic_v2_pb2, cynic_v2_pb2_grpc

CYNIC_ADDR = os.getenv("CYNIC_ADDR", "localhost:50051")


class CynicClient:
    def __init__(self):
        self._channel = grpc.insecure_channel(CYNIC_ADDR)
        self._memory = cynic_v2_pb2_grpc.CognitiveMemoryStub(self._channel)

    def store_fact(self, key: str, content: str, confidence: float) -> bool:
        req = cynic_v2_pb2.Fact(key=key, content=content, confidence=confidence)
        ack = self._memory.StoreFact(req)
        return ack.success

    def close(self):
        self._channel.close()
```

- [ ] Écrire les tests d'intégration (CYNIC doit tourner sur forge)
```python
# tests/test_cynic_client.py
import pytest

@pytest.mark.live
def test_store_fact_reaches_cynic():
    from kairos.cynic_client import CynicClient
    client = CynicClient()
    ok = client.store_fact("test.key", "test content", 0.9)
    assert ok is True
    client.close()
```

> Note : les tests `@pytest.mark.live` ne tournent PAS dans le pipeline validate.sh
> (`-m "not live"`). Ils sont exécutés manuellement quand CYNIC tourne sur forge.

- [ ] Commit
```bash
git add kairos/cynic_client.py protos/ kairos/generated/ tests/test_cynic_client.py
git commit -m "feat(client): KAIROS speaks to CYNIC via gRPC CognitiveMemory"
git push forge main
```

---

## Chunk 5 — Maturité architecturale (futur, post-stabilisation)

> Dette explicite issue de crystallize-truth. À planifier après Chunks 1-4 validés.
> Pas d'implémentation ici — juste les axes identifiés et leur justification.

---

### Axe 1 : Découper cynic.proto en fichiers par service

**Pourquoi :** Le proto monolithique (4 services, ~210 lignes) est copié tel quel dans KAIROS (Task 11). KAIROS n'a besoin que de `CognitiveMemory`, mais reçoit `VascularSystem`, `KPulse`, `MuscleHAL` et tous les messages associés. À chaque agent supplémentaire, la surface inutile se multiplie.

**Cible :**
```
protos/
├── common.proto          # MessageMeta, QScore, EScore, KScore
├── vascular.proto        # VascularSystem service
├── cognitive.proto       # CognitiveMemory service
├── pulse.proto           # KPulse service
└── muscle.proto          # MuscleHAL service
```

**Déclencheur :** Quand un 2e agent (autre que KAIROS) consomme le proto.

---

### Axe 2 : Fermer la boucle K-Pulse

**Pourquoi :** K-Pulse produit des métriques (somatic_score, metabolic_drift, tokens_per_sec) mais rien ne les consomme pour adapter le comportement. La "souveraineté" implique de l'auto-régulation, pas juste du monitoring.

**Cible minimale :** Un actuateur automatique — par exemple :
- Si `metabolic_drift > seuil` → switch de backend (GPU → CPU fallback)
- Si `tokens_per_sec < minimum` → alerte HeresyNotice automatique

**Déclencheur :** Quand un vrai backend d'inférence (non-mock) est intégré.

---

### Axe 3 : QScore / EScore / KScore — exercer ou retirer

**Pourquoi :** Le proto définit trois systèmes de scoring (QScore sur la qualité, EScore sur l'autorité, KScore sur la conviction). Aucun code ne les calcule, compare, ou utilise pour prendre des décisions. C'est de la surface cognitive morte — elle coûte en compréhension sans rien produire.

**Décision à prendre :**
- **Intégrer :** Les scores pilotent des décisions réelles (routing, throttling, trust ranking)
- **Retirer :** Supprimer du proto, réduire la complexité, réintroduire quand le besoin est concret

**Déclencheur :** Avant d'ajouter un 3e service ou agent — la dette conceptuelle doit être résolue.

---

### Axe 4 : Mode dégradé SurrealDB

**Pourquoi :** SurrealDB est un SPOF — si le process tombe, le kernel entier refuse de boot (séquence main.rs : probe → storage → server). C'est un choix acceptable pour du single-node sovereign, mais il doit être explicite et documenté, avec un chemin de dégradation.

**Cible :**
- `CynicStorage::init()` retourne un mode dégradé (in-memory ou read-only cache) si SurrealDB est injoignable
- Le kernel boot quand même, les services signalent `storage_degraded: true` dans les réponses
- SovereigntyAdvisor émet une recommandation de reconnexion

**Déclencheur :** Premier incident de production où SurrealDB down bloque le travail.

---

### Axe 5 : Distribution du proto (CYNIC → agents)

**Pourquoi :** Task 11 copie manuellement `cynic.proto` de CYNIC vers KAIROS (`cp CYNIC-V2/protos/cynic.proto KAIROS/protos/`). Toute modification du proto nécessite une recopie manuelle. Pas de garantie de synchronisation.

**Options :**
- **Git submodule :** `protos/` comme repo partagé — synchronisation via `git submodule update`
- **Crate publié :** Publier les stubs Rust comme crate, les stubs Python comme package
- **Symlink :** Simple mais fragile, ne fonctionne pas cross-platform

**Déclencheur :** Première désynchronisation proto entre CYNIC et KAIROS en production.

---

**Plan révisé. Chunks 1-4 = stabilisation immédiate. Chunk 5 = dette explicite, axes priorisés par déclencheurs concrets.**
