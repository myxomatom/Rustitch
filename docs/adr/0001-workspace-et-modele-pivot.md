# ADR 0001 — Workspace initial et modèle pivot de conversion

- **Statut** : accepté
- **Date** : 2026-10-22
- **Contexte fonctionnel** : CONVERSION §1 (moteur de conversion ; FEATURES §1 à la rédaction), jalon 1.

## Contexte

Le jalon 1 demande un modèle pivot, une quantification sans dérive, la lecture/écriture DST et JEF, une CLI `convert`/`info` et une matrice de tests. Il faut fixer les crates créés, la représentation des données et la frontière entre millimètres et unités machine.

## Décisions

### 1. Seuls les crates nécessaires au jalon sont créés

`emb-core`, `emb-formats`, `emb-cli`. Les autres crates de l'architecture cible (`emb-geometry`, `emb-stitch`, `emb-import`, `emb-render`, `emb-project`, `emb-app`) seront créés avec la première fonctionnalité qui les utilise : un crate vide serait du code mort.

### 2. Modèle pivot en millimètres absolus

`StitchPattern` stocke des positions **absolues** en `Mm(f64)`, une liste de points typés (`StitchKind`), **un fil par bloc de couleur** (invariant vérifié à la construction, champs privés), des métadonnées et une table d'extensions `BTreeMap<String, Vec<u8>>`. Convention d'axes : X vers la droite, **Y vers le haut** — celle des formats machine DST et JEF, ce qui évite toute inversion à leur frontière.

### 3. Représentation machine entière et normalisation exacte

Les écrivains quantifient d'abord (arrondi des positions absolues, fonction unique `quantize`) vers un `MachinePattern` en unités entières, puis appliquent `normalize` selon les `FormatCapabilities` du format. La normalisation travaille donc en arithmétique entière : le découpage des longs déplacements (`floor(d·k/n)`) garantit mathématiquement que chaque déplacement respecte la limite, sans risque d'arrondi flottant, et les points intermédiaires sont répartis régulièrement. Les lecteurs produisent la même représentation entière avant de revenir au pivot. Propriété testée : `lire(écrire(p)) = normaliser(quantifier(p))` pour chaque format.

### 4. Lecteurs et écrivains renvoient des avertissements

Écart assumé par rapport à l'exemple de trait de `CLAUDE.md` : `FormatReader::read` renvoie `ReadOutput { pattern, warnings }` et `FormatWriter::write` renvoie `WriteOutput { warnings }`. La lecture tolérante (CONVERSION §1.5) et le futur rapport de conversion (§1.4) exigent de remonter chaque perte ou approximation de façon typée (`Warning`) plutôt que par des logs.

### 5. CIEDE2000 dès le jalon 1

Écrire un JEF impose de choisir un index de palette ; la distance euclidienne RGB étant interdite (§3.2), CIEDE2000 et la conversion sRGB → Lab sont implémentés et testés (jeu de Sharma) dès maintenant. Le jalon 1 utilise le plus proche voisin **avec signalement** de tout partage d'index entre fils distincts ; l'affectation injective reste au jalon 2.

### 6. Placement JEF : coordonnées conservées par défaut

Le JEF place l'origine au centre du cadre. Par défaut (`Placement::Keep`), Rustitch conserve les coordonnées pour qu'un aller-retour A → JEF → A ne translate rien ; `--center` centre le motif par une translation entière (aucun arrondi supplémentaire).

## Dépendances ajoutées

| Crate | Usage | Justification |
|---|---|---|
| `thiserror` | erreurs des bibliothèques | imposé par `CLAUDE.md` |
| `anyhow` | erreurs de la CLI | imposé par `CLAUDE.md` |
| `clap` (derive) | arguments de la CLI | standard de fait, MIT/Apache-2.0 |
| `proptest` (dev) | tests de propriétés | imposé par `CLAUDE.md` |
| `insta` (dev) | golden files | imposé par `CLAUDE.md` |
| `assert_cmd`, `predicates`, `tempfile` (dev) | tests de bout en bout de la CLI | `assert_cmd` imposé ; les deux autres sont ses compagnons usuels |

Toutes sont sous licences compatibles avec la GPL-3.0.

## Conséquences

- Ajouter un format = un module implémentant `FormatReader`/`FormatWriter` et déclarant ses `FormatCapabilities`, plus une entrée dans `Format`.
- Les spécificités non représentables d'un format iront dans `StitchPattern::extensions` (pas encore utilisée par DST/JEF).
- MSRV 1.85 (édition 2024), vérifiée localement.
