# Travaux identifiés et non faits

Chaque entrée référence la section de [FEATURES.md](FEATURES.md) concernée. Un `TODO` dans le code doit pointer vers une entrée de ce fichier.

## Bloquant pour clore les fonctionnalités DST/JEF du jalon 1

- **Corpus réel** (§9.2, §4.5) : 5 paires DST/JEF reçues le 2026-10-22, dans `tests/fixtures/private/` (non versionné : droits de redistribution inconnus, donc absent de la CI). Testé par `private_corpus`. Manque un corpus redistribuable pour la CI.
- **Bloc `ENHANCE`** : section de type 3 (un `FFFFFFFF` par bloc) et octets `00 00` de tête à comprendre ; acceptation par les machines Janome à vérifier.
- **Codes de cadre JEF 14, 40, 65535** observés et inconnus de la table ; conserver le code lu au lieu de le recalculer.
- **Coupes JEF** : décision du 2026-10-22 (préfixe de saut nul, coupes non écrites et signalées) ; comportement sur machine Janome à vérifier.
- **Drapeaux JEF `0x14` / `0x0A`** et octets 0x16–0x17 : à comprendre et à conserver.
- **Points « à vérifier »** des fiches [dst.md](formats/dst.md) et [jef.md](formats/jef.md) : codes de cadre JEF, convention d'arrêt (index 0), champ `0x0D`, date nulle, seuil de coupe automatique Tajima.
- **Aller-retour JEF octet pour octet sur le corpus** (§4.5) : atteint pour les données de points de la famille `0x0A`, pour les index de couleur et pour le bloc `ENHANCE` ; restent les drapeaux, le code de cadre et les octets 0x16–0x17 de l'en-tête, ainsi que le préfixe des sauts de la famille `0x14`.
- **Recette sur machine Janome** (§4.5) : checklist et fichiers prêts ([acceptance/janome.md](acceptance/janome.md)) ; essai à réaliser.

## Priorité utilisateur : PES → JEF

- **Marque des fils en JEF** : le bloc `ENHANCE` ne la stocke pas (« Madeira Classic 40 » est perdu, signalé). Option envisageable : fiche de fils annexe (§4.3).
- Mode « table » de l'association fils/blocs PES, octets inconnus des fils PES : à confirmer sur d'autres fichiers.
- Lecture des `.pec` seuls (le décodeur PEC existe déjà) ; écriture PES.

## Interface MCP ([ADR 0002](adr/0002-interface-mcp.md))

- ~~Service de conversion partagé~~ : fait (`emb_formats::convert`). Toute nouvelle option passe par `WriteOptions`, jamais par la seule CLI.
- Codes stables et sérialisation JSON des avertissements et du rapport (avec le rapport du jalon 2).
- Crate `emb-mcp` (jalon 4).

## Jalon 2 et suivants

- Mode « préservation du contraste », seuils ΔE configurables et surcharge manuelle des couleurs (§3.4, §4.3).
- Rapport de conversion texte/JSON/HTML (§1.4) — la base (`Warning`) existe.
- Nuanciers commerciaux et tables de correspondance (§3.3).
- Suppression optionnelle des micro-points (§1.3), désactivée par défaut.
- Utilisation de `StitchPattern::extensions` pour un aller-retour sans perte dans le même format (§1.1).
- Écriture des paillettes DST et de l'en-tête étendu `TC:` (option).
- CLI : conversion par lot, `validate`, `diff`, options de couleur/cadre/machine (§5.1).
- Messages de la CLI en français (§9.4).

## Outillage

- Cibles `cargo-fuzz` pour les lecteurs DST et JEF (§4.5, `CLAUDE.md` §5.5). En attendant, des tests `proptest` vérifient l'absence de panique sur octets arbitraires et sur fichiers corrompus.
- Benchmark `criterion` : 100 000 points en moins de 200 ms (§9.1).
- CI : `cargo deny`, `cargo audit`, couverture `cargo llvm-cov` (`CLAUDE.md` §7).
