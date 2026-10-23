# Changelog

Toutes les modifications notables de ce projet sont consignées ici.
Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versionnage [SemVer](https://semver.org/lang/fr/).

## [Unreleased]

### Added

- Workspace Cargo (édition 2024, MSRV 1.85) avec les crates `emb-core`, `emb-formats` et `emb-cli`, lints partagés et CI GitHub Actions (Linux, Windows, macOS).
- `emb-core` : unités `Mm`/`Point` (Y vers le haut), points typés, fils, couleurs sRGB → CIELAB et distance CIEDE2000 (validée sur le jeu de Sharma), horodatages, modèle pivot `StitchPattern` avec invariants vérifiés.
- `emb-formats` : quantification par arrondi des positions absolues, représentation `MachinePattern`, `FormatCapabilities` et normalisation (découpage régulier des longs déplacements, ancrage des commandes, conversion des arrêts et paillettes, coupes implicites), détection du format par signature.
- Lecture et écriture DST (Tajima) et JEF (Janome) tolérantes, avec avertissements typés pour chaque perte ou approximation ; palette Janome à 78 couleurs ; choix automatique du cadre JEF.
- CLI `emb convert` et `emb info`, codes de sortie 0 / 3 (avertissements) / 1 (échec) / 2 (usage).
- Tests : propriétés d'aller-retour, déterminisme, erreur de quantification ≤ 0,05 mm sur 100 000 points, absence de panique sur entrées arbitraires, matrice de conversion DST/JEF, golden files, fixture asymétrique « F ».
- Documentation : fiches `docs/formats/dst.md` et `docs/formats/jef.md`, ADR 0001, `docs/TODO.md`.
- Tests sur un corpus réel privé (`tests/fixtures/private/`, ignoré par git) : lecture, cohérence des paires DST/JEF, réécriture sans perte de points, données de points JEF réécrites à l'identique.

- Lecture des fichiers PES (Brother, toutes versions) : points du bloc PEC (axe Y inversé, design centré), fils réels (RGB, référence, nom, marque) de l'en-tête PES v5+, palette PEC à 64 couleurs sinon. Détection par signature et commande `emb convert x.pes y.jef`.
- `MachinePattern::centered`, `Format::WRITABLE` ; avertissements `ThreadListUnreadable` et `UnknownVersion`.
- Affectation optimale injective des fils à une palette (`emb_core::match_to_palette`, Kuhn-Munkres, déterministe), utilisée par défaut par l'écrivain JEF ; option `emb convert --colors injective|nearest`.

- JEF : lecture et écriture du bloc `ENHANCE` (fils réels : couleur, référence numérique, nom), écrit par défaut (`JefWriter::enhance`, `emb convert --no-enhance`) ; conservation des index Janome d'origine (`jef::PALETTE_EXTENSION`) pour les conversions JEF → JEF. Avertissements `ThreadDetailsNotStored`, `ThreadNameAltered`, `UnreadTrailingData`.
- Exigence « MCP native » : FEATURES §5.3 et ADR 0002.
- Recette machine Janome : checklist `docs/acceptance/janome.md` et générateur des 13 fichiers d'essai (`cargo run -p emb-formats --example janome_acceptance`).
- `.gitattributes` : fins de ligne LF sur tous les systèmes.
- JEF : choix manuel du cadre (`JefWriter::hoop`, `WriteOptions::hoop`), y compris un cadre absent de la table connue comme le RE28b (200 × 280 mm).
- Recette adaptée aux cadres SQ20b, RE20b et RE28b : noms de fichiers sur 2 caractères, sondes de codes de cadre (série H), tailles de cadre (série S), designs réels et témoins.
- Service de conversion partagé `emb_formats::convert` (`detect`, `load`, `resolve_target`, `writer`, `convert`, `WriteOptions`, `ConvertError`) : la CLI n'en est plus qu'une façade, prête à être rejointe par le serveur MCP. Comportement de la CLI inchangé.

### Changed

- JEF : le bloc `ENHANCE` est ajouté après le marqueur de fin (golden files `f_glyph_jef`, `long_moves_and_commands_jef` et `near_neutrals_jef` mis à jour : seul ce bloc s'ajoute, l'en-tête et les points sont inchangés).

- JEF : par défaut, deux fils distincts ne reçoivent plus le même index Janome (golden file `near_neutrals_jef` mis à jour : blancs cassés → White et Ivory White, noir et anthracite → Black et Dark Sepia). Le texte de l'avertissement `ThreadApproximated` ne parle plus de « nearest ».
- `jef::palette::nearest_palette_entry` est remplacée par `jef::palette::match_threads`.
- `Format::writer` renvoie une `Option` (PES n'est pas encore inscriptible).
- JEF : chaque série de sauts est précédée d'un saut nul, comme dans les fichiers réels ; ce préfixe est retiré à la lecture. Les coupes ne sont plus encodées en trois sauts nuls (hypothèse infirmée par le corpus) : elles ne sont pas écrites et l'avertissement `TrimsNotStored` les compte. L'avertissement `ZeroLengthJumpsRemoved` disparaît.
