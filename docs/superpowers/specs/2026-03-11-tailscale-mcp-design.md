# tailscale-mcp — Design Spec

> Serveur MCP en Go exposant un reseau Tailscale comme capacites pour les LLM.
> Binaire unique, zero dependance runtime, multi-noeud, production-grade.

---

## Contexte et justification

### Le probleme

Claude Code ne voit que la machine locale. Le developpeur travaille sur un reseau de machines connectees via Tailscale (dev Windows, VM Ubuntu de validation, futur noeud GPU, futurs noeuds edge). Sans visibilite reseau, l'IA est aveugle au-dela de localhost.

### Pourquoi un MCP

Le protocole MCP (JSON-RPC 2.0) est le standard emergent pour exposer des capacites aux LLM. Il est polyglotte par nature : tout client qui parle JSON peut consommer ce serveur. Le MCP est l'adaptateur entre les capacites physiques (machines, reseau) et le contexte de conversation de l'IA.

### Pourquoi Go

Decision cristallisee par analyse objective :

| Critere | Python | Go | Rust |
|---------|--------|-----|------|
| SSH lib | asyncssh (compat Windows incertaine) | golang.org/x/crypto/ssh — production-grade, 12+ ans | russh — "not recommended for production" |
| MCP SDK | Officiel, mature | Officiel, co-maintenu Google | Officiel, jeune (v0.16) |
| Binaire | Necessite runtime Python | Binaire unique, zero dep | Binaire unique |
| Tailscale | Aucune affinite | Tailscale est ecrit en Go | Aucune affinite |
| Cross-compile | N/A | GOOS=linux go build — trivial | Possible, plus complexe |

Rust elimine par sa lib SSH non production-ready. Python elimine par la dependance runtime et l'incertitude asyncssh sur Windows/MSYS2. Go gagne sur les fondamentaux.

### Verites cristallisees (design drivers)

| # | Verite | Impact |
|---|--------|--------|
| T1 | Le MCP est polyglotte par nature (JSON-RPC) — le vrai enjeu est de separer couche MCP de la logique SSH/Tailscale | Architecture 3 couches, fonctions metier sans import MCP |
| T2 | golang.org/x/crypto/ssh est production-grade, la CLI tailscale est inevitable pour la decouverte | SSH via lib native, Tailscale via CLI --json |
| T3 | La CLI tailscale status --json est le seul moyen de decouverte reseau — dependance a isoler | Module dedie avec parsing strict |
| T4 | Les connexions SSH doivent etre lazy et persistantes par noeud | ConnectionManager avec reconnexion automatique |
| T5 | Le SDK MCP est instable sur le long terme — la logique metier ne doit jamais en dependre | Tools MCP = wrappers minces autour de fonctions pures |

---

## Architecture

### Principe : 3 couches strictement separees

```
Couche MCP (transport)
  main.go + mcp/tools.go
  - SDK MCP officiel Go
  - Declaration des 3 tools
  - Traduction MCP types <-> types metier
  - Remplacable si le SDK change

Couche Metier (logique)
  tailscale/ + ssh/ + transfer/
  - Fonctions pures, structs Go
  - N'importe JAMAIS le SDK MCP
  - Testable independamment
  - Reutilisable hors contexte MCP

Couche Infra (primitives)
  golang.org/x/crypto/ssh
  github.com/pkg/sftp
  github.com/kevinburke/ssh_config
  os/exec (tailscale CLI)
  - Libs externes, isolees derriere interfaces
  - Remplacables sans toucher la logique
```

### Structure du projet

```
tailscale-mcp/
  main.go                     # Point d'entree, wiring MCP server stdio
  mcp/
    tools.go                  # 3 tools MCP (wrappers minces)
  tailscale/
    discovery.go              # Parse tailscale status --json -> []Node
    types.go                  # Structs Node, Status
  ssh/
    client.go                 # SSHClient : exec, session management
    config.go                 # Parse ~/.ssh/config -> resolution host/user/key
    manager.go                # ConnectionManager : lazy persistent par noeud
  transfer/
    sftp.go                   # Push/pull fichiers via SFTP
  go.mod
  go.sum
  Makefile                    # build, cross-compile, test
```

### Dependances

| Module | Role | Justification |
|--------|------|---------------|
| github.com/modelcontextprotocol/go-sdk | Serveur MCP | SDK officiel, co-maintenu Google |
| golang.org/x/crypto/ssh | Client SSH | Production-grade, stdlib etendue Go, 12+ ans |
| github.com/pkg/sftp | Transfert SFTP | Mature, standard de facto, utilise connexion SSH existante |
| github.com/kevinburke/ssh_config | Parse ~/.ssh/config | Leger, supporte les patterns Host, IdentityFile, etc. |

Quatre dependances. Pas de framework, pas de magie.

---

## Tools MCP

### 1. ts_status — Decouverte reseau

Description : Liste tous les noeuds du reseau Tailscale avec leur etat.

Input :
```go
type StatusInput struct {
    Node string `json:"node,omitempty" jsonschema:"description=Filtrer par nom de noeud (optionnel)"`
}
```

Output :
```go
type StatusOutput struct {
    Self  NodeInfo   `json:"self" jsonschema:"description=Ce noeud"`
    Peers []NodeInfo `json:"peers" jsonschema:"description=Noeuds distants"`
}

type NodeInfo struct {
    Name     string `json:"name" jsonschema:"description=Nom Tailscale du noeud"`
    IP       string `json:"ip" jsonschema:"description=Adresse IP Tailscale"`
    OS       string `json:"os" jsonschema:"description=Systeme d'exploitation"`
    Online   bool   `json:"online" jsonschema:"description=Noeud joignable"`
    SSHReady bool   `json:"ssh_ready" jsonschema:"description=Host SSH configure dans ~/.ssh/config"`
    SSHUser  string `json:"ssh_user,omitempty" jsonschema:"description=User SSH si configure"`
}
```

Comportement :
1. Execute tailscale status --json
2. Parse le JSON — attention : `Peer` est une `map[string]*PeerStatus` (keyed par cle publique WireGuard), pas un slice. Iterer la map, extraire HostName de chaque PeerStatus. IP = TailscaleIPs[0] (gerer le cas vide).
3. Cross-reference avec ~/.ssh/config : match exact sur le nom court du noeud Tailscale (ex: "forge") contre les Host entries de la config SSH. Si match, marque ssh_ready: true et extrait le user. Pas de wildcard matching — seul le match exact est fiable.
4. Si node est specifie, filtre le resultat
5. Retourne la liste enrichie

### 2. ts_exec — Execution distante

Description : Execute une commande sur un noeud distant via SSH.

Input :
```go
type ExecInput struct {
    Node    string `json:"node" jsonschema:"required,description=Nom du noeud Tailscale ou alias SSH"`
    Command string `json:"command" jsonschema:"required,description=Commande a executer"`
    Timeout int    `json:"timeout,omitempty" jsonschema:"minimum=1,maximum=3600,default=120,description=Timeout en secondes"`
}
```

Output :
```go
type ExecOutput struct {
    Stdout     string `json:"stdout" jsonschema:"description=Sortie standard"`
    Stderr     string `json:"stderr" jsonschema:"description=Sortie d'erreur"`
    ExitCode   int    `json:"exit_code" jsonschema:"description=Code de retour"`
    DurationMs int64  `json:"duration_ms" jsonschema:"description=Duree d'execution en millisecondes"`
}
```

Comportement :
1. Resout le noeud : cherche dans ~/.ssh/config d'abord (Host alias), puis dans la liste Tailscale (par nom)
2. Recupere ou cree la connexion SSH via ConnectionManager (lazy, persistante)
3. Ouvre une session SSH sur la connexion existante
4. Execute la commande : lance session.Run() dans une goroutine, session.Close() sur timeout du context. stdout et stderr draines dans des bytes.Buffer par des goroutines concurrentes (obligatoire pour eviter le deadlock par buffer exhaustion du channel SSH).
5. Capture stdout et stderr separement (partiels si timeout)
6. Retourne le resultat structure avec le code de retour et la duree

Resolution de noeud :
```
ts_exec("forge", "cargo test")
  1. ~/.ssh/config a "Host forge" -> HostName <TAILSCALE_FORGE>, User kairos, IdentityFile ~/.ssh/kairos_proxmox
  2. Connexion SSH vers kairos@<TAILSCALE_FORGE> avec la cle
  3. Execute "cargo test", capture output
```

### 3. ts_transfer — Transfert de fichiers

Description : Transfere des fichiers entre la machine locale et un noeud distant via SFTP.

Input :
```go
type TransferInput struct {
    Node        string `json:"node" jsonschema:"required,description=Nom du noeud Tailscale ou alias SSH"`
    Source      string `json:"source" jsonschema:"required,description=Chemin source (local si push distant si pull)"`
    Destination string `json:"destination" jsonschema:"required,description=Chemin destination (distant si push local si pull)"`
    Direction   string `json:"direction" jsonschema:"required,description=Direction du transfert,enum=push,enum=pull"`
}
```

Output :
```go
type TransferOutput struct {
    BytesTransferred int64 `json:"bytes_transferred" jsonschema:"description=Octets transferes"`
    FilesTransferred int   `json:"files_transferred" jsonschema:"description=Nombre de fichiers transferes"`
    DurationMs       int64 `json:"duration_ms" jsonschema:"description=Duree en millisecondes"`
}
```

Comportement :
1. Resout le noeud (meme logique que ts_exec)
2. Recupere la connexion SSH via ConnectionManager
3. Ouvre un client SFTP sur la connexion existante
4. Valide les chemins : source doit exister, pas de traversal (../)
5. Si source est un fichier : transfert simple
6. Si source est un repertoire : walk recursif, recreation de l'arborescence
7. Limite hard : 500MB max par transfert, timeout 10 minutes. Au-dela, erreur explicite.
8. Retourne les statistiques de transfert

---

## Composants internes

### tailscale/discovery.go

```go
// Discover appelle tailscale status --json et retourne l'etat du reseau.
func Discover() (*Status, error)

// DiscoverNode retourne un noeud specifique par nom.
func DiscoverNode(name string) (*NodeInfo, error)
```

Parse la sortie JSON de la CLI Tailscale. Attention aux structures reelles :
- `Self` : objet `*ipnstate.Status.Self` (nom, IP dans TailscaleIPs[0], OS)
- `Peer` : `map[key.NodePublic]*PeerStatus` — c'est une **map** keyed par cle publique WireGuard, PAS un slice. L'implementation doit iterer la map et extraire HostName/DNSName de chaque PeerStatus.
- `TailscaleIPs` est `[]netip.Addr` — prendre `[0]` avec gestion du cas vide.

La CLI Tailscale est appelee via `exec.Command("tailscale", "status", "--json")` avec args separes (pas de shell intermediaire). Detection du binaire :
1. `exec.LookPath("tailscale")` (Linux/macOS)
2. `exec.LookPath("tailscale.exe")` (Windows)
3. Fallback Windows : chemins connus (`C:\Program Files\Tailscale\tailscale.exe`)
4. Si introuvable, erreur explicite avec message d'installation.

### ssh/config.go

```go
// ResolveHost cherche un host dans ~/.ssh/config et retourne ses parametres.
func ResolveHost(alias string) (*HostConfig, error)

type HostConfig struct {
    Hostname      string   // HostName resolu
    User          string   // User SSH
    Port          int      // Port (defaut 22)
    IdentityFiles []string // Chemins des cles privees
}
```

Utilise github.com/kevinburke/ssh_config pour parser ~/.ssh/config. Resout les directives :
- HostName (IP ou hostname reel)
- User
- Port
- IdentityFile (peut etre multiple)

Attention Windows/MSYS2 :
- La lib retourne les IdentityFile **verbatim** (ex: `~/.ssh/kairos_proxmox`). L'implementation DOIT expander `~` manuellement via `os.UserHomeDir()` — l'OS ne le fait pas.
- La lib ne supporte pas la directive `Match`. Si le fichier SSH config contient des blocs `Match`, ils seront ignores (pas d'erreur, juste pas resolus). Documenter cette limite.

### ssh/client.go

```go
// Connect cree une connexion SSH vers un host resolu.
// Utilise net.DialTimeout pour la connexion TCP, puis ssh.NewClientConn
// pour le handshake SSH, les deux bornes par un context.WithTimeout.
// Cela evite le bug connu golang/go#51926 ou ssh.Dial peut bloquer
// indefiniment si le daemon SSH est non-responsif apres TCP connect.
func Connect(ctx context.Context, cfg *HostConfig) (*ssh.Client, error)

// Exec execute une commande sur une connexion existante.
// IMPORTANT : ssh.Session.Run() ne supporte pas context. L'implementation
// doit lancer session.Run() dans une goroutine et appeler session.Close()
// quand le context expire. stdout et stderr sont draines dans des bytes.Buffer
// via des goroutines concurrentes pour eviter le deadlock par buffer exhaustion
// du channel SSH (stdout et stderr partagent un buffer fixe).
func Exec(ctx context.Context, client *ssh.Client, command string) (*ExecResult, error)
```

Auth : lit les cles privees depuis les IdentityFiles de la config SSH (avec expansion tilde). Supporte ed25519, RSA, ECDSA. Verification des host keys via ~/.ssh/known_hosts (knownhosts.New de golang.org/x/crypto).

### ssh/manager.go

```go
// ConnectionManager gere les connexions SSH persistantes par noeud.
type ConnectionManager struct {
    connections map[string]*ssh.Client
    mu          sync.RWMutex
}

// Get retourne une connexion existante ou en cree une nouvelle.
func (m *ConnectionManager) Get(alias string) (*ssh.Client, error)

// Close ferme toutes les connexions.
func (m *ConnectionManager) Close()
```

- Lazy : Connexion creee au premier appel Get() pour un noeud donne
- Persistante : Reutilisee pour tous les appels suivants au meme noeud
- Reconnexion : Si la connexion est morte (SendRequest echoue), en cree une nouvelle.
  IMPORTANT : la logique reconnexion doit etre entierement sous write lock (mu.Lock)
  avec re-check apres acquisition du lock pour eviter la race condition ou deux
  goroutines detectent une connexion morte, acquierent le lock sequentiellement,
  et creent chacune une nouvelle connexion (fuite de connexion). Pattern :
  1. RLock -> check alive -> si OK, return
  2. RUnlock -> Lock -> re-check (une autre goroutine a peut-etre deja reconnecte)
  3. Si toujours morte -> reconnecte sous le write lock -> Unlock
- Thread-safe : sync.RWMutex pour acces concurrent
- Timeout de connexion : Connect() est bornee par context (30s par defaut) pour
  eviter qu'un ssh.Dial bloquant ne verrouille le manager pour tous les noeuds

### transfer/sftp.go

```go
// Push envoie un fichier ou repertoire vers un noeud distant.
func Push(client *ssh.Client, source, destination string) (*TransferResult, error)

// Pull recupere un fichier ou repertoire depuis un noeud distant.
func Pull(client *ssh.Client, source, destination string) (*TransferResult, error)
```

Utilise github.com/pkg/sftp sur la connexion SSH existante (pas de nouvelle connexion). Supporte :
- Fichier unique
- Repertoire recursif (walk + mkdir + copy)
- Preservation des permissions

---

## Configuration MCP (cote client)

Le MCP est configure dans ~/.claude/mcp.json (ou .mcp.json projet) :

```json
{
  "mcpServers": {
    "tailscale": {
      "command": "tailscale-mcp",
      "args": []
    }
  }
}
```

Zero configuration du MCP lui-meme. Il decouvre tout depuis :
- Tailscale : tailscale status --json (reseau)
- SSH : ~/.ssh/config (credentials)
- Known hosts : ~/.ssh/known_hosts (verification)

---

## Build et distribution

### Makefile

```makefile
VERSION=$(shell git describe --tags --always --dirty)
LDFLAGS=-ldflags "-X main.version=$(VERSION)"

# Detecte Windows vs Unix pour l'extension binaire
ifeq ($(OS),Windows_NT)
    BINARY=tailscale-mcp.exe
else
    BINARY=tailscale-mcp
endif

build:
	go build $(LDFLAGS) -o $(BINARY) .

build-linux:
	GOOS=linux GOARCH=amd64 go build $(LDFLAGS) -o tailscale-mcp-linux-amd64 .

build-all: build build-linux

test:
	go test ./... -v -race

test-integration:
	go test ./... -v -race -tags integration

clean:
	rm -f tailscale-mcp tailscale-mcp-linux-amd64 tailscale-mcp.exe

install:
	go install $(LDFLAGS) .
```

### Cross-compilation

```bash
# Pour la machine Windows (dev)
go build -o tailscale-mcp.exe .

# Pour forge (Ubuntu)
GOOS=linux GOARCH=amd64 go build -o tailscale-mcp-linux-amd64 .
scp tailscale-mcp-linux-amd64 forge:~/bin/tailscale-mcp
```

Un binaire par OS. Zero dependance. Copier et executer.

---

## Tests

### Strategie

| Type | Quoi | Comment |
|------|------|---------|
| Unit | Parsing SSH config, parsing Tailscale JSON, resolution de noeud | Fixtures JSON, mock fs |
| Integration | Connexion SSH reelle, exec, SFTP | Contre forge via Tailscale (tag integration) |
| Smoke | Le binaire demarre et repond au handshake MCP | echo initialize JSON pipe dans le binaire |

### Tests unitaires (toujours executables)

```go
// tailscale/discovery_test.go
func TestParseStatus(t *testing.T) {
    // Fixture JSON reelle capturee depuis tailscale status --json
    data := loadFixture("testdata/tailscale-status.json")
    status, err := parseStatus(data)
    assert.NoError(t, err)
    assert.Equal(t, "desktop-daulbl9", status.Self.Name)
    assert.Len(t, status.Peers, 1)
    assert.Equal(t, "forge", status.Peers[0].Name)
}

// ssh/config_test.go
func TestResolveHost(t *testing.T) {
    cfg := loadSSHConfig("testdata/ssh_config")
    host, err := resolveHost(cfg, "forge")
    assert.NoError(t, err)
    assert.Equal(t, "<TAILSCALE_FORGE>", host.Hostname)
    assert.Equal(t, "kairos", host.User)
}
```

### Tests d'integration (necessitent le reseau)

```go
//go:build integration

// ssh/client_integration_test.go
func TestRealExec(t *testing.T) {
    client, err := Connect(resolveHost("forge"))
    require.NoError(t, err)
    defer client.Close()

    result, err := Exec(client, "echo hello", 10*time.Second)
    require.NoError(t, err)
    assert.Equal(t, "hello\n", result.Stdout)
    assert.Equal(t, 0, result.ExitCode)
}
```

---

## Gestion d'erreurs

Chaque erreur est categorisee et remontee clairement dans la reponse MCP :

| Erreur | Cause | Reponse MCP |
|--------|-------|-------------|
| Noeud inconnu | Nom ne match ni SSH config ni Tailscale | node 'foo' not found in Tailscale network or SSH config |
| Noeud offline | Tailscale montre le noeud offline | node 'forge' is offline |
| Auth SSH echouee | Cle refusee, user incorrect | SSH auth failed for kairos@<TAILSCALE_FORGE>: permission denied |
| Timeout | Commande depasse le timeout | command timed out after 120s (stdout/stderr partiels retournes) |
| Fichier introuvable | SFTP path invalide | remote file not found: /path/to/file |
| Tailscale pas demarre | Daemon Tailscale inactif | tailscale CLI failed: is the Tailscale daemon running? |

Les erreurs n'interrompent jamais le serveur MCP. Chaque tool retourne son erreur, le serveur continue.

Distinction erreurs MCP vs erreurs Go :
- **Erreur tool** (noeud offline, commande echouee, fichier introuvable) : retourne un `CallToolResult` avec `IsError: true` et le message d'erreur en contenu texte. Le serveur MCP continue normalement. C'est le cas commun.
- **Erreur Go** (return error) : reserve aux erreurs de protocole/transport (MCP SDK internal). Ne devrait jamais arriver en operation normale. Le SDK les remonte comme erreur JSON-RPC au client.

---

## Securite

- Pas de credentials dans le MCP. Tout passe par ~/.ssh/config et les cles SSH existantes.
- Pas de shell injection. Les commandes sont passees directement au binaire (pas de shell intermediaire cote Go, exec.Command avec args separes pour Tailscale CLI). Cote SSH, session.Run() passe la commande au shell distant — c'est le comportement standard SSH.
- Host key verification. Via ~/.ssh/known_hosts — pas de InsecureIgnoreHostKey().
- Perimetre SSH. Le MCP execute avec les permissions du user SSH distant.
- Transport local. MCP stdio = communication locale via pipes. Pas de port reseau expose par le MCP.
- Tailscale. Le reseau Tailscale est un VPN WireGuard chiffre. Les connexions SSH passent dans le tunnel.

---

## Flux de donnees complet

```
Claude Code (local)
  |
  | JSON-RPC stdio
  |
  v
tailscale-mcp (process local)
  |
  |-- ts_status
  |   +-- exec.Command: tailscale status --json
  |       +-- parse JSON -> []NodeInfo
  |           +-- cross-ref ~/.ssh/config
  |               +-- retourne StatusOutput
  |
  |-- ts_exec("forge", "cargo test")
  |   +-- ssh/config: resolve "forge" -> <TAILSCALE_FORGE>, kairos, ~/.ssh/kairos_proxmox
  |       +-- ssh/manager: Get("forge") -> connexion persistante
  |           +-- ssh/client: session.Run("cargo test")
  |               +-- capture stdout, stderr, exit code
  |                   +-- retourne ExecOutput
  |
  +-- ts_transfer("forge", "./file", "/remote/path", "push")
      +-- ssh/manager: Get("forge") -> connexion existante
          +-- sftp.NewClient(conn)
              +-- create remote file, io.Copy
                  +-- retourne TransferOutput
```

---

## Contraintes et limites explicites

- Une seule commande par ts_exec. Pas de multiplexing. Claude fait N appels si N commandes.
- Pas de streaming. stdout/stderr sont retournes en bloc a la fin. Pour les commandes longues, le timeout est le garde-fou.
- Pas de PTY. Les commandes sont executees sans terminal interactif. Les programmes qui necessitent un TTY ne fonctionneront pas.
- Taille des transferts. Limite hard a 500MB par appel, timeout 10 minutes. Au-dela, erreur explicite. Pour le code source et les configs, c'est largement suffisant.
- Pas de cancel mid-transfer. Si le client MCP se deconnecte pendant un transfert SFTP, le transfert se termine (la goroutine continue). C'est acceptable pour des transferts bornes a 500MB.
- Tailscale requis pour ts_status. Le binaire tailscale doit etre dans le PATH (ou dans les chemins connus sur Windows). Sans Tailscale, ts_status echoue mais ts_exec fonctionne si le noeud est dans ~/.ssh/config.
- Directive Match non supportee. Les blocs Match dans ~/.ssh/config sont ignores par la lib kevinburke/ssh_config. Seuls les blocs Host sont resolus.
