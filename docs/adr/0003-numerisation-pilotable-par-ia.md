# ADR 0003 — Recentrage sur la numérisation, pilotable par une IA

- **Statut** : proposé (orientation produit décidée le 2026-11-15 ; traduction en architecture à valider)
- **Date** : 2026-11-15
- **Contexte fonctionnel** : FEATURES §0 (vision), §2 (modèle de design), §6 (pilotage par une IA).
- **Amende** : [ADR 0002](0002-interface-mcp.md) (périmètre et calendrier du serveur MCP).

## Contexte

Les premières spécifications faisaient de la conversion fidèle entre tous les formats la priorité absolue, et rangeaient la numérisation en dernier. La revue des objectifs du 2026-11-15 a établi l'inverse :

- le produit est une **application de numérisation pour non-professionnels** : image à aplats → motif brodable de qualité ;
- sa valeur propre est le **relief donné par le sens et le type de remplissage**, à couleur égale ;
- c'est une **vraie application graphique** (ouvrir, voir, bouton « Numériser », retouche à la souris), pensée dès l'origine pour être **pilotée par une IA** via MCP ;
- la conversion se limite d'abord à **PES → JEF** ; les autres formats viendront sur besoin.

Le code du jalon 1 (modèle pivot, formats, CLI) reste valable : il devient le dernier maillon de la chaîne.

## Décisions

### 1. Deux documents de spécification

`docs/FEATURES.md` décrit le produit ; `docs/CONVERSION.md` reprend les anciennes sections 1 à 5 sur la conversion **sans les renuméroter**, pour que les renvois existants restent justes (« FEATURES §3.4 » devient « CONVERSION §3.4 »).

### 2. Un modèle de design à objets, en amont du modèle pivot

Un design est une liste ordonnée d'objets (forme + type de point + réglages + fil). Les points sont **toujours générés** à partir des objets ; le résultat est le `StitchPattern` de l'ADR 0001, qui reste l'unique entrée des écrivains de formats. Rien ne change dans `emb-formats`.

### 3. Les commandes, point de passage unique

Toute modification d'un design est une commande typée, validée, sérialisable et annulable. L'application graphique et le serveur MCP sont deux **clients minces** de cette couche : ni l'un ni l'autre ne contient de logique métier ni d'accès direct au modèle. Conséquences recherchées :

- la **parité humain / IA** est une propriété de construction, vérifiée par un test (toute commande est exposée en MCP) ;
- l'annulation couvre de la même façon les actions de l'humain et celles de l'IA ;
- le serveur MCP reste petit, comme le voulait l'ADR 0002.

### 4. Le serveur MCP expose le design, pas seulement la conversion

L'ADR 0002 prévoyait `list_formats`, `info`, `convert`, `match_colors`, au jalon 4. Le serveur expose désormais aussi la lecture structurée du design, les commandes d'édition, l'aperçu en image et les diagnostics (FEATURES §6.1), et il est **avancé au jalon 3**, avant l'application : construire le client le plus simple en premier garantit qu'aucune logique ne se loge dans l'interface graphique. Les principes de l'ADR 0002 sont conservés : crate `emb-mcp` séparé, résultats structurés à codes stables, garde-fous d'accès aux fichiers.

### 5. L'aperçu est une bibliothèque sans interface

Le rendu réaliste (éclairage dépendant du sens du fil) produit une image à partir d'un design, sans dépendance graphique (`emb-render`). Il sert à l'application, au serveur MCP (l'IA doit voir son travail) et aux tests (golden files).

### 6. Un socle déterministe sans IA

Le bouton « Numériser » applique des règles intégrées, déterministes, sans connexion. L'IA ajoute les choix d'auteur par-dessus, par les mêmes commandes. L'application ne dépend d'aucun service extérieur.

## Ce que cet ADR ne décide pas

Chacun de ces points fera l'objet de son propre ADR, au jalon indiqué dans FEATURES §12 : bibliothèque d'interface graphique, format du fichier projet, méthode de vectorisation des images, mécanisme de connexion de l'IA à l'application ouverte, appel éventuel d'une IA depuis l'application.

## Conséquences

- Nouveaux crates à créer avec leur première fonctionnalité (règle de l'ADR 0001) : `emb-geometry`, `emb-stitch`, `emb-import`, `emb-render`, `emb-project` (design, commandes, fichier projet), `emb-mcp`, `emb-app`.
- Les formats autres que PES, JEF et DST sortent du plan (CONVERSION §2.2), ainsi que le JEF+.
- La validation ne peut plus être seulement automatique : chaque jalon de numérisation se clôt par une recette sur tissu (FEATURES §10.2).
- La recette du JEF sur la machine d'essai devient la première chose à faire : toute la chaîne en dépend.
