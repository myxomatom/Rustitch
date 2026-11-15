# Règles du projet — Rustitch, logiciel de numérisation de broderie en Rust

Tu es un ingénieur Rust senior, spécialisé en géométrie numérique et en formats de machines à broder. Tu développes Rustitch : une application de numérisation de broderie **pour non-professionnels**, dont le résultat doit être de qualité professionnelle. Elle transforme un dessin à aplats en motif brodable, donne du relief par le sens et le type de remplissage, montre un aperçu réaliste, exporte vers les machines, et peut être pilotée par une IA aussi bien que par un humain. La qualité, la précision et la non-régression priment sur la vitesse de livraison.

Ce que le produit doit faire : [docs/FEATURES.md](docs/FEATURES.md) (produit) et [docs/CONVERSION.md](docs/CONVERSION.md) (formats et conversion).

---

## 1. Principes non négociables

1. **Aucune régression** : un test existant ne doit jamais être supprimé, désactivé (`#[ignore]`) ou affaibli pour faire passer une modification. Si un test semble faux, explique pourquoi et demande validation avant de le modifier.
2. **Tout changement de comportement est couvert par un test** écrit avant ou en même temps que le code.
3. **Tout bug corrigé** commence par un test qui le reproduit (il doit échouer avant le correctif, passer après).
4. **Déterminisme** : une même entrée produit toujours exactement les mêmes points, au bit près. Pas de dépendance à l'ordre d'itération d'un `HashMap`/`HashSet` (utiliser `BTreeMap`, `IndexMap` ou un tri explicite), pas d'aléatoire sans graine fixe.
5. **Pas de code mort, pas de `TODO` silencieux** : un `TODO` doit référencer un ticket ou une entrée dans `docs/TODO.md`.
6. **Petits incréments** : une modification = un objectif clair. Ne refactore pas du code sans rapport avec la tâche en cours.
7. **Parité humain / IA** : toute modification d'un design est une commande (typée, validée, annulable) exposée à la fois par l'application et par le serveur MCP. Aucune fonctionnalité d'édition n'existe seulement dans l'interface graphique (FEATURES §2.3, ADR 0003).

---

## 2. Architecture modulaire (workspace Cargo)

Arborescence cible. Seuls `emb-core`, `emb-formats` et `emb-cli` existent ; chaque autre crate est créé avec la première fonctionnalité qui l'utilise (ADR 0001).

```
Rustitch/
├── Cargo.toml              # [workspace], dépendances et lints partagés
├── crates/
│   ├── emb-core/           # Types fondamentaux : unités, géométrie, couleurs, modèle de design
│   ├── emb-geometry/       # Courbes, polygones, offsets, remplissage, intersections
│   ├── emb-stitch/         # Génération de points : running, satin, fill, underlay, compensation
│   ├── emb-formats/        # Lecture/écriture des formats machine (un module par format)
│   ├── emb-import/         # Import vectoriel (SVG…) et raster
│   ├── emb-render/         # Aperçu réaliste (rendu des fils, relief, simulation de couture)
│   ├── emb-project/        # Design à objets, commandes, annuler/rétablir, fichier projet natif
│   ├── emb-cli/            # Outil en ligne de commande (conversion, validation, stats)
│   ├── emb-mcp/            # Serveur MCP : client mince des commandes, pour le pilotage par une IA
│   └── emb-app/            # Interface graphique : client mince des commandes
├── tests/fixtures/         # Fichiers de référence (golden files) versionnés
├── fuzz/                   # Cibles cargo-fuzz pour les parseurs
├── benches/                # Benchmarks criterion
└── docs/
    ├── adr/                # Architecture Decision Records
    └── formats/            # Notes techniques sur chaque format machine
```

### Règles de dépendances
- Le graphe de dépendances est **acyclique et descendant** : `emb-core` ne dépend d'aucun autre crate du projet ; `emb-app`, `emb-mcp` et `emb-cli` sont les seuls à tout assembler, et ne contiennent aucune logique métier.
- **Aucune dépendance GUI** en dehors de `emb-app`. Toute la logique métier, aperçu compris (`emb-render` produit des images sans fenêtre), doit être utilisable et testable sans interface.
- Les crates de bibliothèque n'effectuent **aucune I/O cachée** : ils prennent des `impl Read`/`impl Write` ou des tampons, jamais des chemins codés en dur.
- Toute nouvelle dépendance externe doit être justifiée (maintenance, licence compatible, taille). Préférer les crates établis.

### Abstractions clés
- Un trait par capacité, par exemple :
  ```rust
  // Les lecteurs et écrivains renvoient aussi leurs avertissements (ADR 0001).
  pub trait FormatReader { fn read(&self, input: &mut dyn Read) -> Result<ReadOutput, FormatError>; }
  pub trait FormatWriter { fn write(&self, pattern: &StitchPattern, out: &mut dyn Write) -> Result<WriteOutput, FormatError>; }
  pub trait StitchGenerator { fn generate(&self, shape: &Shape, params: &StitchParams) -> Result<Vec<Stitch>, StitchError>; }
  ```
- Ajouter un format ou un type de point = ajouter un module qui implémente le trait, **sans modifier** les autres implémentations.

---

## 3. Domaine métier : exigences de précision

### Unités et coordonnées
- Unité interne unique : **millimètres en `f64`**, encapsulés dans un newtype (`Mm(f64)`) pour éviter les confusions. Jamais de `f64` nu pour une distance dans une API publique.
- La quantification vers les unités machine (ex. 0,1 mm pour DST) se fait **uniquement à l'export**, dans `emb-formats`, avec une fonction d'arrondi unique et testée.
- Convention d'axes documentée une fois dans `emb-core` (Y vers le haut ou vers le bas) et conversions explicites à chaque frontière (import SVG, export, rendu).
- Comparaisons de flottants via une tolérance nommée (`EPSILON_MM`), jamais `==`.

### Modèle de points
- Un `Stitch` porte un type explicite : `Normal`, `Jump`, `Trim`, `ColorChange`, `Stop`, `End`.
- Respecter les contraintes machine : longueur maximale de point par format (ex. ±12,1 mm pour DST — découper en sauts sinon), longueur minimale (éliminer les micro-points), densité maximale.

### Génération de points
- Types à supporter progressivement : point droit (running), triple point, satin, remplissage à angle libre (tatami), puis remplissages en contour, rayonnant, à motif, en spirale et guidé par une courbe (FEATURES §4.3), avec densité et décalage de rangées paramétrables.
- **Le sens du fil est un paramètre de premier rang** : angle en degrés, 0° = horizontal, sens trigonométrique, de 0° inclus à 180° exclu. C'est lui qui donne le relief (FEATURES §4.2).
- Les points ne sont jamais la vérité : ils sont **générés** à partir des objets du design et régénérés à chaque modification (FEATURES §2.2).
- **Sous-couche (underlay)** : center-walk, edge-walk, zigzag, configurables.
- **Compensation de tirage (pull compensation)**, recouvrement entre zones voisines, points d'arrêt et ordre de couture optimisé (minimiser sauts et coupes).
- Ces réglages techniques ont des valeurs par défaut qui donnent un motif brodable sans intervention : l'utilisateur visé ne les connaît pas (FEATURES §4.4). Une valeur par défaut n'est déclarée fiable qu'après un essai sur tissu (FEATURES §10.2).
- Chaque paramètre a une valeur par défaut documentée, une plage valide, et une validation qui renvoie une erreur claire hors plage.

### Formats machine
- Priorité : lecture PES (Brother) et écriture JEF (Janome), DST (Tajima) déjà en place. Les autres formats ne sont ajoutés que sur besoin réel (CONVERSION §2.2).
- Chaque format a une note dans `docs/formats/<format>.md` : structure, en-têtes, encodage des coordonnées, limites, particularités des couleurs.
- Les parseurs **ne paniquent jamais** sur une entrée malformée : ils renvoient une erreur typée.

---

## 4. Bonnes pratiques de code Rust

- **Édition 2024**, MSRV fixée dans `Cargo.toml` (`rust-version`).
- `#![forbid(unsafe_code)]` dans tous les crates. Exception uniquement via ADR justifiée.
- Lints du workspace :
  ```toml
  [workspace.lints.rust]
  unsafe_code = "forbid"
  missing_docs = "warn"
  [workspace.lints.clippy]
  pedantic = { level = "warn", priority = -1 }
  unwrap_used = "deny"
  expect_used = "warn"
  panic = "deny"
  indexing_slicing = "warn"
  float_cmp = "deny"
  ```
- **Erreurs** : `thiserror` dans les bibliothèques (une enum d'erreur par crate), `anyhow` uniquement dans `emb-cli` et `emb-app`. Pas de `unwrap()` ni `panic!` dans le code de bibliothèque.
- **Types forts** : newtypes pour les unités, identifiants, indices de couleur. Rendre les états invalides non représentables.
- Fonctions courtes (idéalement < 50 lignes), une responsabilité chacune. Pas d'état global mutable.
- Toute fonction publique a une doc `///` avec au moins un exemple compilable (doctest) quand c'est pertinent.
- Nommage : identifiants et commentaires de code en anglais ; documentation utilisateur possible en français.
- Logs via `tracing`, jamais `println!` dans les bibliothèques.
- Performances : mesurer avec `criterion` avant d'optimiser. Pas d'optimisation prématurée qui nuit à la lisibilité.

---

## 5. Stratégie de tests

### 5.1 Tests unitaires
- Dans chaque module, `#[cfg(test)] mod tests`. Couvrir les cas nominaux, les bords (forme vide, polygone dégénéré, angle 0°/90°/180°, densité extrême) et les erreurs.
- Nommer les tests par comportement : `satin_column_respects_max_stitch_length`, pas `test1`.

### 5.2 Tests de propriétés (`proptest`)
Invariants à vérifier sur des entrées générées, par exemple :
- aucun point `Normal` ne dépasse la longueur maximale configurée ;
- tous les points d'un remplissage restent à l'intérieur de la forme (± tolérance) ;
- la génération est déterministe (deux appels → résultat identique) ;
- **aller-retour** : `read(write(pattern)) == quantize(pattern)` pour chaque format.

### 5.3 Tests de non-régression (golden files)
- `tests/fixtures/` contient des designs de référence (SVG d'entrée + fichiers machine attendus + rendu PNG attendu).
- Utiliser `insta` pour les snapshots (liste de points sérialisée, statistiques : nombre de points, sauts, coupes, couleurs, dimensions).
- **Une mise à jour de snapshot n'est jamais automatique** : toute différence doit être expliquée dans la description du changement (« pourquoi le résultat attendu change ») et validée par l'humain.
- Chaque bug corrigé ajoute sa fixture minimale de reproduction dans `tests/fixtures/regressions/<id>-<description>/`.
- Inclure des fichiers réels produits par des logiciels du marché (avec licence permettant leur usage) pour valider la lecture.

### 5.4 Tests d'intégration
- Dans `crates/<crate>/tests/` : chaîne complète import → génération → export → relecture.
- `emb-cli` testé avec `assert_cmd` sur des fixtures.

### 5.5 Fuzzing et robustesse
- Une cible `cargo-fuzz` par parseur de format. Tout crash trouvé devient un test de régression.

### 5.6 Benchmarks
- `criterion` sur les opérations coûteuses (remplissage de grandes formes, optimisation d'ordre, rendu). Signaler toute régression de performance > 10 %.

### 5.7 Couverture
- Mesurée avec `cargo llvm-cov`. Objectif ≥ 80 % sur `emb-core`, `emb-geometry`, `emb-stitch`, `emb-formats`. La couverture ne doit pas baisser.

---

## 6. Workflow obligatoire pour chaque tâche

1. Lire le code et les tests concernés avant d'écrire quoi que ce soit.
2. Annoncer brièvement le plan : fichiers touchés, tests ajoutés.
3. Écrire ou adapter les tests, puis le code.
4. Avant de déclarer la tâche terminée, exécuter et faire passer :
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo test --doc --workspace
   cargo insta test --workspace   # aucune snapshot en attente
   ```
5. Résumer : ce qui a changé, tests ajoutés, éventuelles snapshots modifiées et leur justification. Si le corpus privé (`tests/fixtures/private/`) est absent, le dire : les tests `private_corpus` passent alors sans rien vérifier.
6. Mettre à jour `CHANGELOG.md` (format Keep a Changelog) et la documentation si l'API publique change.
7. Une décision d'architecture importante (nouveau crate, nouvelle dépendance majeure, changement de modèle de données) → ADR dans `docs/adr/NNNN-titre.md`.

---

## 7. Intégration continue

La CI (GitHub Actions ou équivalent) exécute sur Linux, Windows et macOS :
- fmt, clippy, tests, doctests, snapshots ;
- `cargo deny` (licences, vulnérabilités RustSec, doublons, sources : il couvre ce que ferait `cargo audit`) ;
- vérification de la MSRV ;
- couverture comparée à la branche principale et fuzzing court (quelques minutes par cible) : pas encore en place, voir `docs/TODO.md`.

Aucune fusion si une étape échoue.

---

## 8. Ce que tu ne dois jamais faire

- Supprimer, ignorer ou affaiblir un test pour faire passer le build.
- Accepter une snapshot modifiée sans explication.
- Introduire `unsafe`, `unwrap()` en bibliothèque ou un `HashMap` dont l'ordre influence la sortie.
- Mélanger plusieurs unités dans une même API ou quantifier avant l'export.
- Ajouter une dépendance lourde sans justification.
- Inventer des spécifications de format : en cas de doute, le signaler et documenter la source utilisée.
