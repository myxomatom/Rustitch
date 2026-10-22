# Recette sur machine Janome

Objectif : vérifier sur une vraie machine que les fichiers JEF écrits par Rustitch sont acceptés et brodés fidèlement, et trancher les points « à vérifier » de [jef.md](../formats/jef.md) (FEATURES §4.5).

## 1. Préparer les fichiers

Depuis la racine du projet :

```powershell
cargo build --release
cargo run --release -p emb-formats --example janome_acceptance -- recette-janome/synthetiques
```

- `recette-janome/synthetiques/` : 13 fichiers de test dessinés par Rustitch (partageables).
- `recette-janome/reels/` : vos designs convertis, par exemple :
  `target\release\emb.exe convert "tests\fixtures\private\pes\Love1.pes" "recette-janome\reels\Love1.depuis-pes.jef"`.
  Le fichier `.avertissements.txt` à côté de chacun liste ce que la conversion a approché ou perdu.
- **Témoins** : les JEF d'origine de `tests/fixtures/private/jef/`, produits par d'autres logiciels. Ils servent de référence : si un témoin pose le même problème qu'un fichier Rustitch, ce n'est pas Rustitch qui est en cause.

Copiez les trois dossiers sur la clé USB (ou le support de la machine).

## 2. Noter la machine

| Information | Valeur |
|---|---|
| Modèle |  |
| Version du firmware |  |
| Cadres disponibles |  |
| Support utilisé (USB, câble, logiciel de transfert) |  |
| Date de l'essai |  |

## 3. Fichiers synthétiques

Pour chaque fichier : le charger, observer l'écran, puis broder si la colonne « Broder » l'indique (une chute de tissu et un fil quelconque suffisent, sauf pour les couleurs).

| Fichier | Ce qu'on vérifie | Résultat attendu | Broder | Résultat (OK / KO + remarque) |
|---|---|---|---|---|
| `01-orientation.jef` | Sens des axes | Un **F** à l'endroit (pas en miroir) et une **flèche vers le haut de l'écran**, à droite du F | oui |  |
| `02a-cadre-50x50.jef` | Code de cadre 1 | Accepté ; cadre proposé **50 × 50** (ou plus grand si absent) | non |  |
| `02b-cadre-110x110.jef` | Code de cadre 0 | Cadre **110 × 110** (ou SQ14 / plus grand) | non |  |
| `02c-cadre-126x110.jef` | Code de cadre 3 | Cadre **126 × 110** (ou plus grand) | non |  |
| `02d-cadre-140x200.jef` | Code de cadre 2 | Cadre **140 × 200** (ou plus grand) | non |  |
| `02e-cadre-200x200.jef` | Code de cadre 4 | Cadre **200 × 200** ; refusé si la machine n'a pas ce cadre | non |  |
| `03a-couleurs-avec-enhance.jef` | Bloc `ENHANCE` accepté | Chargé sans erreur ; **6 couleurs** Janome, dans l'ordre : Peach (212), Burnt Orange (235), Baby Blue (228), Peacock Green (251), Olive Green (219), Black (002) | oui (6 fils, ou changer de fil à chaque arrêt) |  |
| `03b-couleurs-sans-enhance.jef` | Même design sans le bloc | Affichage identique à 03a | non |  |
| `04-sauts-coupes.jef` | Coupes automatiques | Noter, pour chaque rangée (sauts de **3 mm** en haut, **15 mm** au milieu, **60 mm** en bas), si la machine **coupe le fil** avant le saut | oui |  |
| `05-points-longs.jef` | Points longs | Zigzags de 5, 10 et 12,7 mm brodés ; en bas, un trait de 20 mm (écrit comme un saut suivi d'un point) | oui |  |
| `06a-centrage-conserve.jef` | Placement par défaut | Carré de 2 cm **décalé** en haut à droite du centre du cadre | non |  |
| `06b-centrage-centre.jef` | Option `--center` | Même carré, **centré** dans le cadre | non |  |
| `07-sans-date.jef` | Date à zéro dans l'en-tête | Chargé sans erreur (même dessin que 01) | non |  |

Questions à noter en plus :

- Quel **nom** de couleur la machine affiche-t-elle pour `03a` (nom Janome de la palette, ou nom du fil réel comme « Orange Sorbet ») ?
- La machine affiche-t-elle un **nombre de points** ou des **dimensions** ? Les relever pour 01 et 03a.

## 4. Designs réels

Pour chacun : charger le JEF converti **et** son témoin, comparer à l'écran, puis broder au moins `Love1.depuis-pes.jef`.

| Fichier | À comparer avec le témoin | Résultat (OK / KO + remarque) |
|---|---|---|
| `Love1.depuis-pes.jef` | Dimensions (96 × 98,6 mm), nombre de couleurs (7), cadre proposé, couleurs affichées ; broderie : texte « Love » lisible, rendu fidèle |  |
| `3 inch 33.depuis-pes.jef` | Dimensions (53,9 × 76,3 mm), 8 couleurs |  |
| `Labubu-2_XL.depuis-pes.jef` | Dimensions (106,8 × 180 mm), 7 couleurs |  |
| `OAD_2002_Floral.Sally.Bunny_KC.depuis-pes.jef` | Dimensions (40,4 × 89,8 mm), 14 couleurs |  |
| `narrow-sport-satin_A_3.5in.depuis-pes.jef` | Dimensions (50,2 × 89 mm), 3 couleurs |  |
| `*.depuis-dst.jef` (5 fichiers) | Même chose. Le DST ne contient pas de couleur : les couleurs sont arbitraires. `3 inch 33.depuis-dst.jef` doit apparaître **décentré** (son DST d'origine l'est) |  |

Pour la broderie de Love1, noter en particulier :

- les **coupes** de fil entre les éléments : la machine coupe-t-elle là où le témoin coupe ?
- les **fils sautés** restés visibles qu'il faudrait couper à la main ;
- tout écart de forme ou de position par rapport au témoin brodé (si vous l'avez brodé).

## 5. Rendre les résultats

Le plus simple : me copier les tableaux remplis (ou une photo), avec le modèle de la machine. J'en tirerai :

- la table des codes de cadre (02a à 02e, et les codes 14, 40, 65535 des témoins) ;
- la décision sur les coupes en JEF (04, Love1) ;
- la validation du bloc `ENHANCE` (03a) et du sens des axes (01) ;
- les corrections éventuelles, chacune avec un test de non-régression.

Les résultats seront consignés dans `docs/acceptance/resultats/` (modèle de machine, firmware, date).
