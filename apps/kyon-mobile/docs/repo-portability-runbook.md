# Kyon Repo Portability Runbook

Objectif: intégrer Kyon dans un nouveau repo sans régression d’architecture.

## Temps cible

- 30–60 min pour un repo Android existant
- 10–20 min pour un repo vierge

## Checklist (obligatoire)

1. Copier la topologie `core/`, `ports/`, `application/`, `adapters/`, `bootstrap/`, `ui/`, `mirror/`, `shield/`.
2. Déplacer `KyonApp` dans le nouveau package root et garder `bootstrap` comme point unique de composition.
3. Ajouter les permissions minimum puis services Android nécessaires.
4. Brancher un `ObservationSink` réel ou mock in-memory pour démarrage local.
5. Paramétrer `KernelConfig`/feature flags via préférences (pas de constantes hardcodées).
6. Configurer base de données et nommage local (ex: `<org>.db`).
7. Brancher WorkManager / foreground constraints si nécessaire.
8. Ajouter tests unitaires d’au moins:
   - `RecordNotificationEvent`
   - `ShieldDecision`
   - `FlushPendingEvents`
9. Exécuter:
   - `./gradlew :app:compileDebugKotlin`
   - `./gradlew :app:testDebugUnitTest`
10. Documenter la dérive dans `README.md` du nouveau repo.

## Quick migration map

Remplacement d’adaptateurs obligatoires:

- `RoomActivityEventStore` → adapter storage du repo cible
- `RoomPhoneReportStore` → adapter storage du repo cible
- `KernelObservationSink` → adapter réseau/IPC du repo cible
- `KernelSyncWorker` → worker local du repo cible

Reste à ne pas réécrire:

- Les cas d’usage (`application/*`) et ports (`ports/*`) sauf extension.
- Les modèles de domaine (`core/model/*`) sauf adaptation de schéma.

## Validation de portabilité

- Pas de dépendance de `application` vers Android frameworks (`Context`, `Room`,
  `WorkManager`, `SharedPreferences`).
- `KyonApp` ou l’équivalent réinitialise toutes les dépendances à partir de `bootstrap`.
- Les tests existent au niveau de `application`.
- La configuration runtime est externalisée.

## Script de démarrage (template)

```bash
cp -r apps/kyon-mobile/app/src/main/java/so/cynic/kyon/* <new_repo>/app/src/main/java/<new_pkg>/
cp apps/kyon-mobile/app/build.gradle.kts <new_repo>/app/build.gradle.kts
cp -r apps/kyon-mobile/app/src/main/res <new_repo>/app/src/main/
```

Puis:

```bash
# ajuster package names + manifest + dependencies
rg -l "so\\.cynic\\.kyon" <new_repo>/app/src/main/java | xargs sed -i 's/so\\.cynic\\.kyon/<new_pkg>/g'
```

Le `sed` de remplacement doit rester limité au code, puis tu répares manuellement
les éventuels noms de classe externes.

## Décision d’acceptation

Portabilité validée quand:
- le runbook est documenté dans le nouveau repo,
- la compilation Kotlin passe,
- le flux de sync peut être désactivé / réactivé par config,
- la détection de permission et la capture d’événements restent explicites et opt-in.

