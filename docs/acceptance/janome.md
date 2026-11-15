# Recette sur machine Janome

Objectif : vérifier sur une vraie machine que les fichiers JEF écrits par Rustitch sont acceptés et brodés fidèlement, et trancher les points « à vérifier » de [jef.md](../formats/jef.md) (FEATURES §4.5).

Machine visée : cadres **SQ20b** (200 × 200 mm), **RE20b** (170 × 200 mm) et **RE28b** (200 × 280 mm). Noms de fichiers sur **2 caractères**, sans caractère spécial.

## 1. Préparer la clé USB

Depuis la racine du projet :

```powershell
cargo run --release -p emb-formats --example janome_acceptance -- recette-janome/USB (Get-ChildItem tests/fixtures/private/pes/*.pes, tests/fixtures/private/dst/*.dst).FullName
```

puis copier les JEF d'origine comme témoins `T1` à `T5` (fait automatiquement lors de la préparation du 2026-10-23).

- `recette-janome/USB/` : **le contenu à copier tel quel sur la clé** (40 fichiers).
- `recette-janome/USB.txt` : ce que contient chaque fichier, et ce que la conversion a approché ou perdu. À garder sur l'ordinateur.

**Code de cadre.** Les codes JEF des cadres SQ20b, RE20b et RE28b ne sont pas encore connus. Sauf mention contraire, chaque fichier déclare le **code 4** (200 × 200 dans la table de pyembroidery). La série `H` sert à découvrir les vrais codes.

## 2. Noter la machine

| Information | Valeur |
|---|---|
| Modèle |  |
| Version du firmware |  |
| Support utilisé (clé USB, câble, logiciel) |  |
| Date de l'essai |  |

## 3. Série H : quel cadre pour quel code ?

Même petit carré de 30 mm, seul le code de cadre change. Pour chaque fichier, **charger seulement** et noter le cadre que la machine affiche ou demande (SQ20b, RE20b, RE28b, autre nom, message d'erreur…).

| Fichier | Code écrit | Cadre affiché par la machine |
|---|---|---|
| `H0` | 0 |  |
| `H1` | 1 |  |
| `H2` | 2 |  |
| `H3` | 3 |  |
| `H4` | 4 |  |
| `H5` | 5 |  |
| `H6` | 6 |  |
| `H7` | 7 |  |
| `H8` | 8 |  |
| `HA` | 14 |  |
| `HB` | 40 |  |
| `HC` | 65535 |  |

Noter aussi le cadre affiché pour les témoins : `T1` (code 14), `T2` (code 40), `T3` (code 65535), `T4` (code 0), `T5` (code 2).

## 4. Série S : tailles de vos cadres

Rectangles qui remplissent presque chaque cadre, avec une croix au centre. **Charger**, noter le cadre proposé et si la machine accepte ; broder `S1` si possible (fil quelconque) pour vérifier que le rectangle tient dans le SQ20b.

| Fichier | Taille du motif | Code écrit | Attendu | Résultat |
|---|---|---|---|---|
| `S1` | 196 × 196 mm | 4 | Accepté dans le **SQ20b** |  |
| `S2` | 166 × 196 mm | 4 | Accepté dans le **RE20b** (ou le SQ20b) |  |
| `S3` | 196 × 276 mm | 4 | Ne tient que dans le **RE28b** : noter si la machine le propose ou refuse le fichier |  |
| `S4` | 196 × 276 mm | 65535 | Même motif que S3, autre code : comparer la réaction |  |

## 5. Série 01 à 08 : fonctions

| Fichier | Ce qu'on vérifie | Résultat attendu | Broder | Résultat (OK / KO + remarque) |
|---|---|---|---|---|
| `01` | Sens des axes | Un **F** à l'endroit (pas en miroir) et une **flèche vers le haut de l'écran**, à droite du F | oui |  |
| `02` | Bloc `ENHANCE` accepté | Chargé sans erreur ; **6 couleurs** Janome, dans l'ordre : Peach (212), Burnt Orange (235), Baby Blue (228), Peacock Green (251), Olive Green (219), Black (002) | oui (changer de fil à chaque arrêt) |  |
| `03` | Même design sans le bloc | Affichage identique à 02 | non |  |
| `04` | Coupes automatiques | Pour chaque rangée (sauts de **3 mm** en haut, **15 mm** au milieu, **60 mm** en bas), noter si la machine **coupe le fil** avant le saut | oui |  |
| `05` | Points longs | Zigzags de 5, 10 et 12,7 mm brodés ; en bas, un trait de 20 mm (écrit comme un saut suivi d'un point) | oui |  |
| `06` | Placement par défaut | Carré de 2 cm **décalé** en haut à droite du centre du cadre | non |  |
| `07` | Option « centrer » | Même carré, **centré** dans le cadre | non |  |
| `08` | Date à zéro dans l'en-tête | Chargé sans erreur (même dessin que 01) | non |  |

À noter en plus : le **nom** de couleur affiché pour `02` (nom Janome, ou nom du fil réel comme « Orange Sorbet ») et, si la machine les affiche, le nombre de points et les dimensions de `01` et `02`.

## 6. Série R : vos designs convertis

`R1` à `R5` viennent des **PES**, `R6` à `RA` des **DST** (sans couleur : couleurs arbitraires). Comparer chacun à son témoin `T`, puis broder au moins `R3`.

| Fichier | Design (source) | Témoin | À comparer | Résultat |
|---|---|---|---|---|
| `R1` | 3 inch 33 (PES) | `T1` | 53,9 × 76,3 mm, 8 couleurs |  |
| `R2` | Labubu (PES) | `T2` | 106,8 × 180 mm, 7 couleurs |  |
| `R3` | Love1 (PES) | `T3` | 96 × 98,6 mm, 7 couleurs ; **broder** : texte « Love » lisible, rendu fidèle |  |
| `R4` | OAD Floral (PES) | `T4` | 40,4 × 89,8 mm, 14 couleurs |  |
| `R5` | narrow-sport (PES) | `T5` | 50,2 × 89 mm, 3 couleurs |  |
| `R6` | 3 inch 33 (DST) | `T1` | Doit apparaître **décentré** (son DST d'origine l'est) |  |
| `R7` à `RA` | Labubu, Love1, OAD, narrow-sport (DST) | `T2` à `T5` | Mêmes dimensions que les témoins |  |
| `RB` | Birds 6x10 (PES) | — | 141,5 × 251,9 mm, 19 couleurs (dont 4 arrêts sur le même noir) ; ne tient que dans le **RE28b** (déclaré avec le code 4, comme `S3`) : noter si la machine l'accepte |  |

Pour la broderie de `R3`, noter :

- les **coupes** de fil entre les éléments : la machine coupe-t-elle là où le témoin `T3` coupe ?
- les **fils sautés** restés visibles, qu'il faudrait couper à la main ;
- tout écart de forme ou de position par rapport au témoin brodé (si vous l'avez brodé).

## 7. Rendre les résultats

Le plus simple : me copier les tableaux remplis (ou une photo), avec le modèle de la machine. J'en tirerai :

- la table des codes de cadre (séries H et S, témoins) et l'ajout du SQ20b, du RE20b et du RE28b à Rustitch ;
- la décision sur les coupes en JEF (04, R3) ;
- la validation du bloc `ENHANCE` (02) et du sens des axes (01) ;
- les corrections éventuelles, chacune avec un test de non-régression.

Les résultats seront consignés dans `docs/acceptance/resultats/` (modèle de machine, firmware, date).
