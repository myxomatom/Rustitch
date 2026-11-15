# Contribuer à Rustitch

Les règles de code et de tests sont dans [CLAUDE.md](CLAUDE.md) ; les fonctionnalités attendues dans [docs/FEATURES.md](docs/FEATURES.md). Ce document décrit le circuit de travail.

## Circuit d'une modification

1. Partir de `main` à jour et créer une branche : `git switch -c <sujet>`.
2. Écrire les tests, puis le code (CLAUDE.md §6).
3. Vérifier en local avant de pousser :

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo test --doc --workspace
   cargo insta test --workspace --all-features --check --unreferenced=reject
   cargo deny check
   ```

4. Pousser la branche et ouvrir une pull request vers `main` ; remplir le modèle (objectif, tests, snapshots).
5. La CI se lance automatiquement. La PR ne peut être fusionnée que lorsque la vérification **CI** est verte.
6. Fusionner (bouton « Merge pull request »), puis supprimer la branche.

`main` est protégée : pas de poussée directe, pas de réécriture d'historique, pas de fusion sans CI verte.

## Ce que vérifie la CI

| Job | Contenu |
|---|---|
| `test (ubuntu-latest)`, `test (windows-latest)`, `test (macos-latest)` | Formatage, clippy (avertissements = erreurs), tests, doctests, golden files `insta` (aucune snapshot en attente ni orpheline) |
| `msrv (1.85)` | Compilation de tout le code et des tests avec la version minimale de Rust |
| `cargo-deny` | Licences compatibles GPL-3.0, failles connues (base RustSec), crates retirées, provenance des dépendances |
| `CI` | Récapitulatif : vert seulement si tous les jobs ci-dessus le sont ; c'est la vérification exigée pour fusionner |

Une nouvelle poussée sur une PR annule l'exécution en cours. Dependabot ouvre chaque semaine des PR de mise à jour des dépendances et des actions, soumises à la même CI.

Les tests sur fichiers réels (`tests/fixtures/private/`) ne tournent qu'en local : ces fichiers ne sont pas redistribuables et ne sont pas dans le dépôt.

## Snapshots

Une snapshot ne se met jamais à jour automatiquement. Après `cargo insta review`, chaque différence acceptée est expliquée dans la PR (CLAUDE.md §5.3).
