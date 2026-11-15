# Travaux identifiés et non faits

Chaque entrée référence la section de [FEATURES.md](FEATURES.md) (produit) ou de [CONVERSION.md](CONVERSION.md) (formats) concernée. Un `TODO` dans le code doit pointer vers une entrée de ce fichier. Les jalons sont dans FEATURES §11.

## 1. À faire en premier : recette sur la machine d'essai (jalon 1)

Toute la chaîne se termine par un JEF : tant qu'il n'est pas validé sur machine, le reste repose sur des hypothèses.

- **Recette sur machine** (CONVERSION §4.5) : checklist et fichiers prêts ([acceptance/janome.md](acceptance/janome.md)) ; essai à réaliser. Noter le modèle exact de la machine (« eXpressive », famille Janome) et la version de son logiciel interne.
- Points que la recette doit trancher :
  - **Codes de cadre JEF 14, 40, 65535** observés et inconnus de la table ; codes des cadres SQ20b, RE20b et RE28b ; conserver le code lu au lieu de le recalculer.
  - **Coupes JEF** : décision du 2026-10-22 (préfixe de saut nul, coupes non écrites et signalées) ; comportement de la machine à vérifier.
  - **Bloc `ENHANCE`** : acceptation par la machine ; section de type 3 (un `FFFFFFFF` par bloc) et octets `00 00` de tête à comprendre.
  - **Points « à vérifier »** des fiches [dst.md](formats/dst.md) et [jef.md](formats/jef.md) : convention d'arrêt (index 0), champ `0x0D`, date nulle, seuil de coupe automatique Tajima.

## 2. Reste du jalon 1 : conversion PES → JEF

- **Corpus réel** (CONVERSION §6.2, §4.5) : 5 paires DST/JEF reçues le 2026-10-22, leurs PES jumeaux et `Birds6x10.PES`, dans `tests/fixtures/private/` (non versionné : droits de redistribution inconnus, donc absent de la CI). Testé par `private_corpus`. Manque un corpus redistribuable pour la CI.
- **Drapeaux JEF `0x14` / `0x0A`** et octets 0x16–0x17 : à comprendre et à conserver.
- **Aller-retour JEF octet pour octet sur le corpus** (CONVERSION §4.5) : atteint pour les données de points de la famille `0x0A`, pour les index de couleur et pour le bloc `ENHANCE` ; restent les drapeaux, le code de cadre et les octets 0x16–0x17 de l'en-tête, ainsi que le préfixe des sauts de la famille `0x14`.
- **Marque des fils en JEF** : le bloc `ENHANCE` ne la stocke pas (« Madeira Classic 40 » est perdu, signalé). Option envisageable : fiche de fils annexe (CONVERSION §4.3).
- Mode « table » de l'association fils/blocs PES, octets inconnus des fils PES : à confirmer sur d'autres fichiers.
- **Rapport de conversion** texte/JSON/HTML (CONVERSION §1.4), avec codes stables et sérialisation JSON des avertissements ([ADR 0002](adr/0002-interface-mcp.md)) — la base (`Warning`) existe. Prérequis du serveur MCP.
- Service de conversion partagé : fait (`emb_formats::convert`). Règle à tenir : toute nouvelle option passe par `WriteOptions`, jamais par la seule CLI.

## 3. Numérisation : décisions à prendre avant de coder

Voir FEATURES §12. Chaque décision d'architecture donne lieu à un ADR ; l'[ADR 0003](adr/0003-numerisation-pilotable-par-ia.md) (recentrage) est à valider.

- Ordre des jalons 3 (socle IA), 4 (application) et 5 (images et bouton « Numériser »).
- Jalon 2 : représentation des formes et des objets, algorithmes de remplissage et de satin, sources métier pour les valeurs par défaut (densité, compensation, sous-couches) ; liste et dessin des motifs de référence (FEATURES §10.2).
- Jalon 3 : format du fichier projet ; SDK MCP (`rmcp`) à justifier.
- Jalon 4 : bibliothèque d'interface graphique.
- Jalon 5 : méthode de vectorisation des images.

## 4. Conversion : plus tard

- Mode « préservation du contraste », seuils ΔE configurables et surcharge manuelle des couleurs (CONVERSION §3.4, §4.3).
- Nuanciers commerciaux et tables de correspondance (CONVERSION §3.3).
- Suppression optionnelle des micro-points (CONVERSION §1.3), désactivée par défaut.
- Utilisation de `StitchPattern::extensions` pour un aller-retour sans perte dans le même format (CONVERSION §1.1).
- Écriture des paillettes DST et de l'en-tête étendu `TC:` (option).
- CLI : conversion par lot, `validate`, `diff`, options de couleur/cadre/machine (CONVERSION §5.1).
- Messages de la CLI en français (FEATURES §10.4).
- Lecture des `.pec` seuls (le décodeur PEC existe déjà) ; écriture PES ; autres formats et JEF+ (CONVERSION §2.2, §4.4) : sur besoin réel seulement.

## 5. Outillage

- **Tests du corpus privé muets** : sans les fichiers de `tests/fixtures/private/` (CI, sessions de développement dans le cloud), les 9 tests `private_corpus` passent sans rien vérifier. Une régression sur les fichiers réels n'y est donc pas détectée. À traiter par un corpus redistribuable (§2) ; en attendant, lancer ces tests en local avant toute fusion qui touche `emb-formats`.
- **Règles de dépendances entre crates vérifiées par la machine** (`CLAUDE.md` §2) : dès la création du quatrième crate, un test sur `cargo metadata` (graphe descendant, aucune logique dans `emb-app`/`emb-mcp`/`emb-cli`) et une interdiction `cargo-deny` des bibliothèques graphiques hors de `emb-app`.
- **Découpage des jalons** : avant de coder un jalon, le découper en tâches d'un objectif chacune, avec critère d'acceptation et tests attendus (`CLAUDE.md` §1.6).
- Cibles `cargo-fuzz` pour les lecteurs DST, JEF et PES (CONVERSION §4.5, `CLAUDE.md` §5.5). En attendant, des tests `proptest` vérifient l'absence de panique sur octets arbitraires et sur fichiers corrompus.
- Benchmark `criterion` : 100 000 points en moins de 200 ms (CONVERSION §6.1).
- CI : couverture `cargo llvm-cov` et fuzzing court (`CLAUDE.md` §7). `cargo-deny` (licences, failles RustSec, doublons, sources) est en place ; il couvre aussi ce que ferait `cargo audit`.
- PR Dependabot sur `dtolnay/rust-toolchain` : elle remplace la version 1.85 du job `msrv` ; à refuser ou à exclure de Dependabot, sinon la MSRV n'est plus vérifiée.
