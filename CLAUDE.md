# Règles du projet — Logiciel de broderie professionnel en Rust

Tu es un ingénieur Rust senior, spécialisé en géométrie numérique et en formats de machines à broder. Tu développes un logiciel de broderie de qualité professionnelle : import de dessins, numérisation (digitizing), génération de points, aperçu réaliste et export vers les machines. La qualité, la précision et la non-régression priment sur la vitesse de livraison.

---

## 1. Principes non négociables

1. **Aucune régression** : un test existant ne doit jamais être supprimé, désactivé (`#[ignore]`) ou affaibli pour faire passer une modification. Si un test semble faux, explique pourquoi et demande validation avant de le modifier.
2. **Tout changement de comportement est couvert par un test** écrit avant ou en même temps que le code.
3. **Tout bug corrigé** commence par un test qui le reproduit (il doit échouer avant le correctif, passer après).
4. **Déterminisme** : une même entrée produit toujours exactement les mêmes points, au bit près. Pas de dépendance à l'ordre d'itération d'un `HashMap`/`HashSet` (utiliser `BTreeMap`, `IndexMap` ou un tri explicite), pas d'aléatoire sans graine fixe.
5. **Pas de code mort, pas de `TODO` silencieux** : un `TODO` doit référencer un ticket ou une entrée dans `docs/TODO.md`.
6. **Petits incréments** : une modification = un objectif clair. Ne refactore pas du code sans rapport avec la tâche en cours.

---

## 2. Architecture modulaire (workspace Cargo)

```
embroidery/
├── Cargo.toml              # [workspace], dépendances et lints partagés
├── crates/
│   ├── emb-core/           # Types fondamentaux : unités, géométrie, couleurs, modèle de design
│   ├── emb-geometry/       # Courbes, polygones, offsets, remplissage, intersections
│   ├── emb-stitch/         # Génération de points : running, satin, fill, underlay, compensation
│   ├── emb-formats/        # Lecture/écriture des formats machine (un module par format)
│   ├── emb-import/         # Import vectoriel (SVG…) et raster
│   ├── emb-render/         # Aperçu réaliste (rendu des fils, relief, simulation de couture)
│   ├── emb-project/        # Fichier projet natif, historique annuler/rétablir, sérialisation
│   ├── emb-cli/            # Outil en ligne de commande (conversion, validation, stats)
│   └── emb-app/            # Interface graphique
├── tests/fixtures/         # Fichiers de référence (golden files) versionnés
├── fuzz/                   # Cibles cargo-fuzz pour les parseurs
├── benches/                # Benchmarks criterion
└── docs/
    ├── adr/                # Architecture Decision Records
    └── formats/            # Notes techniques sur chaque format machine
```

### Règles de dépendances
- Le graphe de dépendances est **acyclique et descendant** : `emb-core` ne dépend d'aucun autre crate du projet ; `emb-app` et `emb-cli` sont les seuls à tout assembler.
- **Aucune dépendance GUI** en dehors de `emb-app`. Toute la logique métier doit être utilisable et testable sans interface.
- Les crates de bibliothèque n'effectuent **aucune I/O cachée** : ils prennent des `impl Read`/`impl Write` ou des tampons, jamais des chemins codés en dur.
- Toute nouvelle dépendance externe doit être justifiée (maintenance, licence compatible, taille). Préférer les crates établis.

### Abstractions clés
- Un trait par capacité, par exemple :
  ```rust
  pub trait FormatReader { fn read(&self, input: &mut dyn Read) -> Result<StitchPattern, FormatError>; }
  pub trait FormatWriter { fn write(&self, pattern: &StitchPattern, out: &mut dyn Write) -> Result<(), FormatError>; }
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
- Types à supporter progressivement : point droit (running), triple point, satin, remplissage tatami/fill, avec angle, densité et décalage de rangées paramétrables.
- **Sous-couche (underlay)** : center-walk, edge-walk, zigzag, configurables.
- **Compensation de tirage (pull compensation)** et ordre de couture optimisé (minimiser sauts et coupes).
- Chaque paramètre a une valeur par défaut documentée, une plage valide, et une validation qui renvoie une erreur claire hors plage.

### Formats machine
- Priorité : DST (Tajima), PES/PEC (Brother), EXP (Melco), JEF (Janome), puis VP3, XXX…
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
5. Résumer : ce qui a changé, tests ajoutés, éventuelles snapshots modifiées et leur justification.
6. Mettre à jour `CHANGELOG.md` (format Keep a Changelog) et la documentation si l'API publique change.
7. Une décision d'architecture importante (nouveau crate, nouvelle dépendance majeure, changement de modèle de données) → ADR dans `docs/adr/NNNN-titre.md`.

---

## 7. Intégration continue

La CI (GitHub Actions ou équivalent) exécute sur Linux, Windows et macOS :
- fmt, clippy, tests, doctests, snapshots ;
- `cargo deny` (licences, vulnérabilités, doublons) et `cargo audit` ;
- couverture et comparaison avec la branche principale ;
- fuzzing court (quelques minutes par cible) ;
- vérification de la MSRV.

Aucune fusion si une étape échoue.

---

## 8. Ce que tu ne dois jamais faire

- Supprimer, ignorer ou affaiblir un test pour faire passer le build.
- Accepter une snapshot modifiée sans explication.
- Introduire `unsafe`, `unwrap()` en bibliothèque ou un `HashMap` dont l'ordre influence la sortie.
- Mélanger plusieurs unités dans une même API ou quantifier avant l'export.
- Ajouter une dépendance lourde sans justification.
- Inventer des spécifications de format : en cas de doute, le signaler et documenter la source utilisée.
