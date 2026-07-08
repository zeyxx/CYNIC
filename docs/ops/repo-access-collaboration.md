# Collaboration Repo Access (Human + `cynic`)

## Objectif

Permettre à un utilisateur `cynic` dédié à l'organisme de travailler sur les mêmes repos que ton compte humain
sans `sudo`, sans duplication et sans simulators.

## Mécanisme natif

Le CLI dédié `cynic` enregistre maintenant automatiquement le dépôt courant lors d'un
lancement `cynic codex` / `cynic agy` (via `organs/supervisor`), et le stocke dans :

- `infra/repo-access-profile.json`
- la trajectoire runtime `RepoAccessManager` (auditée, reproductible, testable)

Aucun script dédié ne remplace cette mécanique maintenant.
La façade historique n'est plus une entrée opératoire.

## Commande opérante

En flux normal, ajoute un dépôt en direct avec :

```bash
cynic repos add /home/user/Bureau/CAPLOGY/privacy-ethereum-framework-Ihceen
```

Ajouter plusieurs dépôts d'un coup :

```bash
cynic repos add /home/user/Bureau/CAPLOGY/privacy-ethereum-framework /home/user/Bureau/CAPLOGY/privacy-ethereum-framework-Ihceen
```

Ne réintroduis pas de façade shell pour ce flux: l'entrée opératoire reste `cynic repos add`.

## Vérification

### Repos requis (exemple actuel)

- `~/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC`
- `~/Bureau/CAPLOGY/privacy-ethereum-framework`
- `~/Bureau/CAPLOGY/privacy-ethereum-framework-Ihceen` *(le fork de travail)*

Le premier `cynic codex` / `cynic agy` dans un nouveau clone suffit pour enregistrer
automatiquement le chemin dans `infra/repo-access-profile.json`.

## Quand l'accès POSIX est restreint

La façade legacy tente d'utiliser la même logique de projection native. Si l'appel échoue,
la correction minimale reste :

- gérer un groupe Linux commun (créé par `root`),
- basculer le dépôt dans ce groupe avec `chgrp -R` puis `chmod g+ws`.

Ce fallback est moins souple que les ACL par défaut.

## Vérification minimale

```bash
sudo -u cynic git -C /home/user/Bureau/CAPLOGY/privacy-ethereum-framework-Ihceen status
```

(`git` doit maintenant passer sans `Permission denied` côté FS).

Si Git affiche encore une erreur `detected dubious ownership`, exécute ensuite:

```bash
sudo -u cynic git config --global --add safe.directory /home/user/Bureau/CAPLOGY/privacy-ethereum-framework-Ihceen
```
