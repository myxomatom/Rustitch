# ADR 0002 — Interface MCP : un client mince des bibliothèques

- **Statut** : accepté
- **Date** : 2026-10-22
- **Contexte fonctionnel** : FEATURES §5.3 (interface MCP), §1.4 (rapport de conversion).

## Contexte

L'application doit être « MCP native » : un modèle de langage doit pouvoir lister les formats, inspecter un fichier, convertir et choisir des couleurs via le Model Context Protocol, sans passer par un shell ni analyser du texte libre. L'exigence arrive après le jalon 1 : la CLI existe déjà et contient une partie de la logique de conversion.

## Décisions

### 1. Le serveur MCP est un crate séparé, ajouté plus tard

Un crate `emb-mcp` (binaire, transport stdio), au même niveau que `emb-cli` et `emb-app` dans le graphe de dépendances : il assemble les bibliothèques et ne contient **aucune logique métier**. Il utilisera le SDK Rust officiel du protocole (`rmcp`), dont l'ajout fera l'objet d'une justification au moment de l'implémentation (version, licence, taille). Prévu au jalon 4.

### 2. Un service de conversion partagé par la CLI et le serveur

La logique qui était dans `emb-cli` (priorité du contenu sur l'extension pour détecter le format, déduction du format cible, validation des options propres à un format, choix de l'écrivain, date par défaut) est dans le module **`emb_formats::convert`** (fait le 2026-10-22) : `detect`, `load`, `resolve_target`, `writer`, `convert`. Les options y sont des `Option` (`WriteOptions`) : une option donnée pour un format auquel elle ne s'applique pas est une erreur typée (`ConvertError::OptionNotApplicable`), utile aussi pour un modèle qui se tromperait de paramètre. La bibliothèque ne lit pas l'horloge : la date par défaut est fournie par l'appelant.

Choix d'un module plutôt que d'un crate dédié : la logique est petite et ne dépend que des formats ; un crate `emb-service` pourra être extrait si elle grandit (rapport, traitement par lot). La CLI ne garde que les fichiers, l'affichage (dont la traduction des noms d'options en noms de drapeaux) et les codes de sortie ; le serveur MCP fera de même.

### 3. Des résultats lisibles par une machine

Le rapport de conversion (§1.4, jalon 2) est conçu dès le départ pour deux publics :

- chaque `Warning` reçoit un **code stable** en `snake_case` (ex. `thread_approximated`, `trims_not_stored`) en plus de son message ;
- le rapport, les statistiques et les avertissements sont sérialisables en JSON (`serde`), avec un schéma publié ;
- les messages restent pour les humains ; les modèles s'appuient sur les codes et les champs.

### 4. Des garde-fous propres au serveur

- accès aux fichiers limité à des dossiers autorisés par configuration ;
- taille de fichier et durée d'opération bornées ;
- aucun écrasement de fichier sans paramètre explicite ;
- aucune interaction en cours d'opération (le déterminisme est déjà garanti par les bibliothèques).

## Conséquences

- Rien n'est à refaire dans les bibliothèques existantes : elles n'effectuent pas d'I/O cachée, sont déterministes et décrivent déjà chaque perte par un avertissement typé.
- Deux chantiers deviennent des prérequis : le service de conversion partagé et le rapport JSON à codes stables.
- Les nouveaux outils MCP (ex. `validate`, `diff`) suivront les commandes CLI correspondantes, sur le même service.
