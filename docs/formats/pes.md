# Format PES / PEC (Brother)

Implémentation : [`crates/emb-formats/src/pes.rs`](../../crates/emb-formats/src/pes.rs) (en-tête PES) et [`crates/emb-formats/src/pec/`](../../crates/emb-formats/src/pec/) (bloc PEC). **Lecture seulement** pour l'instant.

> **Statut de vérification** : structure recoupée entre pyembroidery et libembroidery (la seconde semble dérivée de la première pour l'en-tête PES : ce ne sont pas deux sources indépendantes), puis **confirmée sur 5 fichiers réels** (versions 1, 7 et 10) dont les jumeaux DST et JEF sont aussi dans le corpus privé.

## Vue d'ensemble

| Propriété | Valeur |
|---|---|
| Unité | 0,1 mm |
| Axes du PEC | X vers la droite, **Y vers le bas** (inversé à la lecture) — **confirmé** par rendu de `Love1.pes` |
| Origine du PEC | Coin haut-gauche du design (boîte englobante de 0 à largeur/hauteur, 5/5) |
| Points | Contenus dans le bloc PEC, présent dans toutes les versions |
| Couleurs | Index de la palette Brother (PEC) ; liste de fils RGB complète dans l'en-tête PES à partir de la version 5 |

Un fichier PES = un en-tête PES (version, objets éditables, fils) + un bloc PEC (la partie « machine »). Rustitch lit les points dans le PEC et les fils dans l'en-tête PES quand il en contient.

## En-tête PES

| Offset | Contenu |
|---|---|
| 0x00 | `#PES` + version sur 4 chiffres (`0001`, `0070`, `0100`…) |
| 0x08 | Offset du bloc PEC (u32) |
| 0x10 | Versions ≥ 4 : nom, catégorie, auteur, mots-clés, commentaires (chaînes préfixées par leur longueur sur 1 octet) |

Versions ≥ 5, après les métadonnées (octets à sauter selon la version, valeurs de pyembroidery) :

| Versions | Avant le nom de cadre | Avant le nom d'image | Après le nom d'image |
|---|---|---|---|
| 5, 5.5, 5.6 | — | 24 | 24 |
| 6, 7 | — | 36 | 24 |
| 8 | — | 38 | 26 |
| 9 | 14 | 30 | 34 |
| 10 | 14 | 38 | 34 |

Puis trois compteurs u16 (remplissages programmables, motifs, plumes) : s'ils ne sont pas nuls, la suite a une structure inconnue et la liste de fils n'est pas lue (avertissement `ThreadListUnreadable`, couleurs de la palette PEC). Sinon : nombre de fils (u16), puis pour chaque fil :

| Champ | Contenu |
|---|---|
| chaîne | Référence catalogue (« 1026 ») |
| 3 octets | Couleur R, G, B |
| 5 octets | Inconnus (toujours 0 dans le corpus) — à vérifier |
| chaîne | Nom (« Orange Sorbet ») |
| chaîne | Marque (« Madeira Classic 40 ») |
| chaîne | Nuancier (vide dans le corpus) |

Association fils → blocs : une entrée par bloc ; sinon, si le nombre de fils égale le nombre d'index PEC distincts, un fil par index dans l'ordre de première apparition (mode « table » de pyembroidery, **non observé** dans le corpus — à vérifier) ; sinon avertissement et palette PEC.

## Bloc PEC (offsets relatifs à son début)

| Offset | Contenu |
|---|---|
| 0 | `LA:` puis nom sur 16 caractères |
| 48 | Nombre de changements de couleur (n − 1 ; `0xFF` = aucune couleur) |
| 49 | n index de palette PEC (1 à 64) |
| 512 | En-tête du bloc de points : `00 00`, offset des vignettes (u24), `31 FF F0`, largeur et hauteur (u16), deux u16 constants (`0x1E0`, `0x1B0` dans le corpus) |
| 528 | Données de points |

### Données de points

| Octets | Signification |
|---|---|
| `FF` | Fin (suivi de `00` dans le corpus) |
| `FE B0 xx` | Changement de couleur (`xx` alterne 1 et 2) |
| sinon | X puis Y, chacun en forme courte ou longue |

- Forme courte (1 octet, bit 7 à 0) : entier signé sur 7 bits.
- Forme longue (2 octets, bit 7 à 1) : entier signé sur 12 bits (`(o1 & 0x0F) << 8 | o2`) ; bit 5 (`0x20`) = coupe, bit 4 (`0x10`) = saut.
- Dans le corpus, les drapeaux sont toujours identiques sur X et Y : `0x10/0x10` (saut), `0x20/0x20` (coupe puis déplacement), ou aucun (point long). Rustitch lit `0x20` comme une coupe à la position courante suivie d'un saut.

## Placement

L'origine du PEC étant le coin du design, le lecteur **centre** le design sur l'origine (translation entière, sans arrondi), ce qui correspond au placement dans le cadre observé dans l'en-tête PES des 5 fichiers (designs centrés dans un cadre 200 × 200).

## Observations sur fichiers réels (corpus du 2026-10-22)

| Point | Constat |
|---|---|
| Versions | 3 × `0001`, 1 × `0070`, 1 × `0100` |
| Liste de fils | Lue sans erreur en v7 et v10 ; marques réelles (Madeira Classic 40) en v7. Les mêmes fils figurent dans le bloc `ENHANCE` du JEF jumeau (le n° de ce bloc est donc la référence catalogue). |
| Coupes | « 3 inch 33 » : 86 coupes explicites, exactement le nombre déduit par la règle « ≥ 3 sauts » du DST jumeau. |
| Taille | Identique (± 0,1 mm) aux jumeaux DST et JEF (5/5). |

## Palette PEC

64 couleurs (index 1 à 64), reprises de pyembroidery `EmbThreadPec.py` (MIT) par extraction automatique. L'index PEC n'est **pas** une référence catalogue Brother : seuls la couleur et le nom sont renseignés. Valeurs RGB à vérifier contre le nuancier Brother.

## Sources

- pyembroidery `PesReader.py`, `PecReader.py`, `EmbThreadPec.py` (MIT) — <https://github.com/EmbroidePy/pyembroidery>.
- libembroidery `src/formats/brother.c` (zlib) — <https://github.com/Embroidermodder/libembroidery> : décodage des points PEC (inversion verticale, flags), en-têtes PES v5–v10.
- Aucun code n'a été copié ; seule la table de palette (données) est reprise, avec attribution.
