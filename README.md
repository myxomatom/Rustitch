# Rustitch
Application de broderie numérique développée en Rust, permettant de concevoir, convertir et préparer des motifs de broderie pour machines à broder. 🧵

## État

Jalon 1 en cours (voir [docs/FEATURES.md](docs/FEATURES.md)) : conversion DST ⇄ JEF et PES → JEF/DST via un modèle pivot, en ligne de commande. Les formats sont vérifiés sur un petit corpus privé de fichiers réels, pas encore sur machine ([docs/TODO.md](docs/TODO.md)).

## Utilisation

```bash
cargo build --release
./target/release/emb info motif.dst
./target/release/emb convert motif.dst motif.jef           # format de sortie déduit de l'extension
./target/release/emb convert motif.dst motif.jef --center  # centre le motif dans le cadre Janome
```

Codes de sortie : `0` succès, `3` succès avec avertissements (approximation ou perte signalée sur la sortie d'erreur), `1` échec, `2` usage invalide.

## Organisation

| Crate | Rôle |
|---|---|
| `crates/emb-core` | Unités, couleurs (CIEDE2000), fils, modèle pivot `StitchPattern` |
| `crates/emb-formats` | Quantification, normalisation, lecteurs/écrivains DST et JEF, lecteur PES |
| `crates/emb-cli` | Outil en ligne de commande `emb` |

Règles de développement : [CLAUDE.md](CLAUDE.md). Circuit de travail (branches, PR, CI) : [CONTRIBUTING.md](CONTRIBUTING.md). Fiches de formats : [docs/formats/](docs/formats/). Décisions d'architecture : [docs/adr/](docs/adr/).
