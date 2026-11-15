# Format JEF (Janome)

Implémentation : [`crates/emb-formats/src/jef/`](../../crates/emb-formats/src/jef/).

> **Statut de vérification** : la structure est recoupée entre pyembroidery et libembroidery, puis comparée à 5 fichiers réels de producteurs inconnus (voir « Observations sur fichiers réels ») ; elle **n'a pas encore été vérifiée sur machine** (CONVERSION §4.1 et §4.5). Les points « à vérifier » sont bloquants pour passer les fonctionnalités JEF au statut ☑.

## Vue d'ensemble

| Propriété | Valeur |
|---|---|
| Unité | 0,1 mm |
| Axes du fichier | X vers la droite, **Y vers le haut** (identiques au modèle pivot) |
| Origine | (0, 0) = centre du cadre |
| Déplacement max. | ±127 par axe (un octet signé ; −128 exclu car 0x80 sert d'échappement) |
| Couleurs | index dans la palette Janome (1 à 78) |

## En-tête (entiers 32 bits petit-boutistes)

| Offset | Taille | Contenu |
|---|---|---|
| 0x00 | 4 | Offset des données de points = `0x74 + 8 × nb_couleurs` |
| 0x04 | 4 | Drapeaux, toujours `0x14` — signification **à vérifier** |
| 0x08 | 14 | Date ASCII `AAAAMMJJhhmmss` |
| 0x16 | 2 | Zéros |
| 0x18 | 4 | Nombre de couleurs (= nombre de blocs) |
| 0x1C | 4 | Nombre de « points » = taille des données de points / 2 (fin incluse) |
| 0x20 | 4 | Code du cadre (voir plus bas) |
| 0x24 | 16 | Emprise depuis le centre : gauche, haut, droite, bas |
| 0x34 | 16 | Distances aux bords du cadre 110 × 110 (g, h, d, b), ou −1 × 4 si le motif dépasse |
| 0x44 | 16 | Idem, cadre 50 × 50 |
| 0x54 | 16 | Idem, cadre 140 × 200 |
| 0x64 | 16 | Idem, cadre « personnalisé » — les deux sources y répètent le 140 × 200 (**à vérifier**) |
| 0x74 | 4 × n | Index de palette de chaque bloc |
| 0x74 + 4n | 4 × n | Type de fil de chaque bloc, toujours `0x0D` — signification **à vérifier** |

### Codes de cadre — **à vérifier**

Les sources divergent ; Rustitch suit pyembroidery (le plus maintenu) :

| Code | Cadre (pyembroidery) |
|---|---|
| 0 | 110 × 110 |
| 1 | 50 × 50 |
| 2 | 140 × 200 |
| 3 | 126 × 110 |
| 4 | 200 × 200 |

libembroidery nomme les cadres `HOOP_126X110`, `HOOP_50X50`, `HOOP_110X110`, `HOOP_140X200` et `HOOP_230X200` (un 230 × 200 là où pyembroidery a un 200 × 200), mais leurs valeurs numériques n'ont pas pu être consultées (fichier d'en-tête introuvable). Ses seuils de sélection diffèrent aussi. Le cadre écrit par Rustitch est **le plus petit qui contient le motif autour du centre** ; si aucun ne convient, l'écriture échoue (`DesignTooLarge`), jamais de troncature.

### Date

Prise dans les métadonnées du design. La bibliothèque ne lit jamais l'horloge système (déterminisme) : sans date, elle écrit `00000000000000` (**à vérifier** : réaction des machines) ; la CLI renseigne la date courante si le design source n'en a pas.

## Données de points

| Octets | Signification |
|---|---|
| `dx dy` (dx ≠ 0x80) | Point normal (octets signés) |
| `80 01 dx dy` | Changement de couleur (Rustitch écrit toujours `dx = dy = 0`, la commande étant ancrée à la position courante) |
| `80 02 dx dy` | Saut |
| `80 10` | Fin |

### Coupes et préfixe des séries de sauts

Le JEF n'a pas de commande de coupe : aucun fichier du corpus réel n'en contient (voir les observations plus bas). Décision du 2026-10-22, en attendant un essai sur machine Janome :

- **Écriture** : chaque série de sauts consécutifs est précédée d'**un** saut nul `80 02 00 00`, comme le fait la famille de fichiers `0x0A`. Les coupes du modèle pivot ne sont pas écrites ; l'avertissement `TrimsNotStored` en donne le nombre. Le comportement de la machine (coupe automatique ou non sur ces sauts) reste **à vérifier**.
- **Lecture** : le saut nul en tête d'une série est retiré (artefact d'encodage) ; tout autre saut nul est conservé.
- **Vérification** : sur les 3 fichiers réels de la famille `0x0A`, les données de points réécrites sont identiques octet pour octet aux originales (test `private_corpus`).

L'ancienne hypothèse « coupe = trois sauts nuls » (option de pyembroidery) est abandonnée : elle n'apparaît dans aucun fichier réel.

### Arrêts — **à vérifier**

pyembroidery encode un arrêt comme un changement de couleur vers l'index 0. Cette convention n'étant confirmée par aucune autre source, Rustitch convertit les arrêts en changements de couleur vers le même fil (avertissement) et signale un index 0 lu comme `UnknownPaletteIndex`.

## Couleurs

- Palette de 78 entrées (couleur, nom, code catalogue Janome) reprise de pyembroidery `EmbThreadJef.py` (MIT), vérifiée par comparaison automatique. Les valeurs RGB sont des approximations — **à vérifier** contre le nuancier officiel Janome.
- Écriture : un fil Janome (marque « Janome » + code catalogue connu) garde son index exact, qui lui est réservé. Les autres fils sont affectés par la stratégie choisie (`JefWriter::colors`, `emb convert --colors`) :
  - **injective** (par défaut) : des fils distincts reçoivent des index distincts, en minimisant la somme des ΔE00 (Kuhn-Munkres, coûts entiers au 1/10 000 de ΔE, égalités départagées par l'index le plus petit) ; s'il y a plus de fils distincts que d'index libres, les fils les plus proches sont regroupés d'abord ;
  - **nearest** : index au ΔE00 minimal pour chaque fil (aperçu rapide).
- Avertissements : `ThreadApproximated` si ΔE00 > 3, `ThreadsShareColor` si des fils distincts partagent un index (stratégie *nearest*, ou plus de fils que de couleurs).
- Validation : sur `Love1.pes` (fils Madeira), 6 index sur 7 identiques à ceux du JEF jumeau produit par un logiciel du marché ; le 7ᵉ (Hint of Mint) hésite entre Baby Blue et Pale Sky.

## Placement

`Placement::Keep` (défaut) conserve les coordonnées pivot : l'origine du design devient le centre du cadre. `Placement::Center` (`emb convert --center`) translate le design d'un nombre entier d'unités pour centrer sa boîte englobante, sans aucun arrondi supplémentaire.

## Observations sur fichiers réels (corpus du 2026-10-22)

Corpus : 5 JEF de `tests/fixtures/private/jef/` (non versionné), chacun avec son DST équivalent. Producteurs encore inconnus ; deux familles se distinguent par le champ de drapeaux.

| Point | Constat | Statut |
|---|---|---|
| Axes | Rendu de `Love1.jef` en Y vers le haut : texte lisible, non inversé. Les sauts JEF et DST d'une même paire atterrissent aux mêmes coordonnées. | **confirmé** |
| Origine | Les 5 designs sont centrés sur l'origine (emprises symétriques). | confirmé |
| Nombre de points (0x1C) | = octets jusqu'au marqueur de fin inclus / 2 (5/5). | **confirmé** |
| Emprises et blocs de bords | Formule confirmée (cadres 110, 50, 140 × 200 répété, −1 si dépassement) sur 5/5. | **confirmé** |
| Type de fil | `0x0D` sur 5/5. | confirmé (sens inconnu) |
| Changement de couleur | Toujours `80 01 00 00` (5/5). | confirmé |
| Drapeaux (0x04) | `0x14` (2 fichiers) ou `0x0A` (3 fichiers) : **pas une constante**. | à comprendre |
| Octets 0x16–0x17 | `00 00` en famille `0x0A` ; non nuls (`6a 00`, `79 00`) en famille `0x14`. | à comprendre |
| Codes de cadre | 0, 2, **14, 40, 65535** observés. 0 et 2 cohérents avec la table pyembroidery ; les trois autres en sont absents. Le code 2 apparaît sur un motif qui tiendrait dans le 110 × 110 : choix de l'utilisateur, pas du plus petit cadre. | table **incomplète** |
| Coupes | L'encodage « 3 sauts nuls » n'apparaît **dans aucun fichier**. Famille `0x0A` : chaque série de sauts commence par **un** saut nul, qu'elle corresponde à une coupe dans le DST jumelé ou non. Famille `0x14` : saut nul seulement avant la première série. Le JEF ne semble porter **aucune coupe explicite**. | hypothèse pyembroidery **infirmée** ; préfixe reproduit à l'identique |

### Bloc `ENHANCE` après le marqueur de fin (famille `0x14`)

Les deux fichiers `0x14` contiennent, après `80 10`, un bloc donnant les **vraies couleurs** des fils. C'est un petit conteneur à sections (entiers 32 bits petit-boutistes) ; les tailles se recalculent exactement sur les deux fichiers (26 + 356 + 36 = 418 et 26 + 312 + 32 = 370 octets).

| Position relative | Contenu |
|---|---|
| +0 | `00 00` (peut-être la fin d'une commande de fin sur 4 octets — à vérifier ; non compté dans le nombre de points de l'en-tête) |
| +2 | ASCII `ENHANCE` |
| +9 | Nombre de sections (1 octet) : `2` |
| +10 | Pour chaque section : type (u32), taille en octets (u32) |
| ensuite | Contenu des sections, dans l'ordre |

| Section | Contenu |
|---|---|
| type 0 — fils | n (u32), puis n enregistrements de 44 octets : index du bloc (u32), couleur **R, G, B, 0**, référence catalogue (u32, 0 si aucune), nom ASCII complété par des zéros (32 octets) |
| type 3 — inconnue | n (u32), puis n × `FF FF FF FF` (sens inconnu — à vérifier) |

Exemples : « Orange Sorbet » `(252,198,141)` n° 1026 (Madeira Classic 40, identique au fil du PES jumeau) pour l'index Janome 18 (Peach) ; « R184 G63 B3 » `(184,63,3)` n° 0 pour un fil sans nom ni référence.

Ce que le bloc **ne contient pas** : la marque du fil, et toute référence non numérique.

#### Prise en charge

- **Lecture** : quand le bloc a un enregistrement par bloc de couleur, ses fils remplacent ceux de la palette (un enregistrement identique à son entrée de palette — même couleur, même code — redevient le fil Janome). Bloc illisible → `ThreadListUnreadable` ; nombre d'enregistrements incohérent → `ThreadCountMismatch` ; autres données après la fin → `UnreadTrailingData`. Dans tous ces cas, couleurs de la palette.
- **Index d'origine** : le lecteur conserve les index Janome de l'en-tête dans `StitchPattern::extensions["jef.palette"]` ; l'écrivain les réutilise s'ils correspondent toujours aux blocs. Une conversion JEF → JEF garde donc les choix de couleur d'origine.
- **Écriture** (par défaut, désactivable par `JefWriter::enhance` / `emb convert --no-enhance`) : un enregistrement par bloc ; un fil sans nom est nommé `R… G… B…` comme dans les fichiers réels ; nom limité à 31 caractères ASCII imprimables (`ThreadNameAltered`) ; marque et référence non numérique perdues (`ThreadDetailsNotStored`).
- **Vérification** : les blocs `ENHANCE` de `Love1.jef` et `3 inch 33.jef`, relus puis réécrits, sont identiques octet pour octet aux originaux (test `private_corpus`).
- **À vérifier sur machine** : qu'un JEF portant ce bloc est accepté. Les fichiers réels qui le portent proviennent de logiciels du marché, ce qui le laisse penser.

## Sources

- pyembroidery `JefReader.py`, `JefWriter.py`, `EmbThreadJef.py` (MIT) — <https://github.com/EmbroidePy/pyembroidery>.
- libembroidery `src/formats/janome.c` (zlib) — <https://github.com/Embroidermodder/libembroidery> : recoupement de l'en-tête (date sur 16 octets, 5 blocs de cadre).
- Aucun code n'a été copié ; seule la table de palette (données) est reprise, avec attribution.
