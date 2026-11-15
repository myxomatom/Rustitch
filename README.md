# Rustitch
Application de numérisation de broderie pour non-professionnels, développée en Rust. 🧵

Rustitch transforme un dessin ou un logo en motif brodable de qualité : le sens et le type de remplissage de chaque zone donnent du relief à l'image, même avec peu de couleurs. L'application est conçue pour être pilotée par une IA (via le Model Context Protocol) aussi bien qu'à la souris. Spécification : [docs/FEATURES.md](docs/FEATURES.md).

## État

Le projet en est au premier de ses jalons ([docs/FEATURES.md §11](docs/FEATURES.md)) : **seule la conversion de fichiers existe**, en ligne de commande (DST ⇄ JEF et PES → JEF/DST via un modèle pivot). Les formats sont vérifiés sur un petit corpus privé de fichiers réels, pas encore sur machine ([docs/TODO.md](docs/TODO.md)). La numérisation, l'aperçu, l'application graphique et le serveur MCP restent à faire.

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

Spécifications : [docs/FEATURES.md](docs/FEATURES.md) (produit) et [docs/CONVERSION.md](docs/CONVERSION.md) (formats et conversion). Règles de développement : [CLAUDE.md](CLAUDE.md). Circuit de travail (branches, PR, CI) : [CONTRIBUTING.md](CONTRIBUTING.md). Fiches de formats : [docs/formats/](docs/formats/). Décisions d'architecture : [docs/adr/](docs/adr/).
