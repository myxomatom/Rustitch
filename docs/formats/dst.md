# Format DST (Tajima)

Implémentation : [`crates/emb-formats/src/dst.rs`](../../crates/emb-formats/src/dst.rs).

> **Statut de vérification** : la structure ci-dessous est confirmée par deux implémentations open source indépendantes, puis comparée à 5 fichiers réels de producteurs inconnus (voir « Observations sur fichiers réels »). Les points encore marqués « à vérifier » restent ouverts.

## Vue d'ensemble

| Propriété | Valeur |
|---|---|
| Unité | 0,1 mm |
| Axes du fichier | X vers la droite, **Y vers le haut** (identiques au modèle pivot : aucune inversion) |
| Origine | (0, 0) = point de départ de l'aiguille |
| Déplacement max. par enregistrement | ±121 unités (12,1 mm) par axe |
| Couleurs | **aucune** dans le format standard ; en-tête étendu `TC:` optionnel (voir plus bas) |
| Coupes | pas de commande dédiée : séquence de sauts |
| Arrêt / changement de couleur | même enregistrement (indiscernables) |

## En-tête (512 octets)

Lignes ASCII `CLÉ:valeur` terminées par `\r` (0x0D), suivies de 0x1A puis complétées par des espaces (0x20) jusqu'à 512 octets.

| Clé | Format écrit | Contenu |
|---|---|---|
| `LA` | `LA:%-16s` | Nom du design, 16 caractères, complété par des espaces |
| `ST` | `ST:%7d` | Nombre d'enregistrements — **à vérifier** : Rustitch écrit le nombre total d'enregistrements de 3 octets, fin incluse ; pyembroidery écrit son propre décompte de points |
| `CO` | `CO:%3d` | Nombre de changements de couleur |
| `+X`, `-X`, `+Y`, `-Y` | `%5d` | Emprise maximale de chaque côté de l'origine (valeurs positives) |
| `AX`, `AY` | `AX:+%5d` / `AX:-%5d` | Position du dernier point par rapport à l'origine |
| `MX`, `MY` | `MX:+    0` | Multi-design, toujours 0 |
| `PD` | `PD:******` | Inutilisé |

En-tête étendu (lecture uniquement) : lignes `AU:` (auteur), `CP:` (copyright) et `TC:#RRGGBB,description,référence` (un fil par bloc). Cette extension provient de pyembroidery ; elle n'est pas standard — **à vérifier** si d'autres logiciels la produisent.

## Enregistrements (3 octets)

Chaque déplacement est codé en ternaire équilibré sur les poids 1, 3, 9, 27, 81 :

| Octet | bit 7 | bit 6 | bit 5 | bit 4 | bit 3 | bit 2 | bit 1 | bit 0 |
|---|---|---|---|---|---|---|---|---|
| 0 | y+1 | y−1 | y+9 | y−9 | x−9 | x+9 | x−1 | x+1 |
| 1 | y+3 | y−3 | y+27 | y−27 | x−27 | x+27 | x−3 | x+3 |
| 2 | saut | couleur | y+81 | y−81 | x−81 | x+81 | 1 | 1 |

Décodage de l'octet 2 (dans cet ordre) :

| Masque | Valeur | Signification |
|---|---|---|
| `& 0xF3` | `0xF3` | Fin |
| `& 0xC3` | `0xC3` | Changement de couleur / arrêt |
| `& 0x43` | `0x43` | Bascule du mode paillettes |
| `& 0x83` | `0x83` | Saut (ou éjection de paillette en mode paillettes) |
| sinon | | Point normal |

## Choix d'implémentation

- **Coupes** : écrites comme trois sauts de déplacement nul au total, `(+2, −2) (−4, +4) (+2, −2)` (repère du fichier), comme pyembroidery. À la lecture, cette signature redevient une coupe.
- **Coupes implicites** : les machines Tajima coupent d'elles-mêmes après une suite de sauts (réglage machine, 3 par défaut ici — **à vérifier** selon les machines). À la lecture, une suite d'au moins 3 sauts sans signature de coupe reçoit une coupe explicite ; à l'écriture, la normalisation ajoute une coupe explicite avant une telle suite (avertissement `ImplicitTrimsInserted`). Ainsi `lire(écrire(p)) = normaliser(quantifier(p))`.
- **Arrêts** : convertis en changements de couleur vers le même fil (avertissement).
- **Paillettes** : lues ; non écrites pour l'instant (converties en sauts, avertissement).
- **Couleurs** : perdues à l'écriture (avertissement `ColorsNotStored`) ; à la lecture sans `TC:`, une palette contrastée par défaut est affectée (avertissement `DefaultColorsAssigned`).
- **Nom** : restreint à l'ASCII imprimable et à 16 caractères (avertissement `NameAltered`).
- **Lecture tolérante** : fichier < 512 octets → erreur ; enregistrement incomplet → lecture jusqu'au dernier enregistrement valide + `TruncatedData` ; absence de fin → `MissingEndMarker`.

## Observations sur fichiers réels (corpus du 2026-10-22)

Corpus : 5 DST de `tests/fixtures/private/dst/` (non versionné), chacun avec son JEF équivalent ; producteurs encore inconnus.

| Point | Constat | Statut |
|---|---|---|
| `ST` | = nombre total d'enregistrements, fin incluse (5/5). | **confirmé** (choix de Rustitch) |
| `AX` / `AY` | = position finale sur 4/5 ; un fichier y recopie `+X`/`+Y`. | confirmé, avec exceptions |
| `+X` … `-Y` | Exacts sur 4/5 ; un fichier décalé d'une unité et formaté `   09`. Le lecteur ne s'en sert pas : correct. | à ne pas exploiter |
| Après la fin | Un octet `0x1A` sur 4/5. | ignoré à la lecture : correct |
| Changement de couleur | Toujours déplacement nul (5/5). | confirmé |
| En-tête `TC:` | Absent des 5 fichiers. | — |
| Coupes | La signature pyembroidery n'apparaît dans aucun fichier. Un producteur ajoute un aller-retour nul `(13,−1)(−13,1)` avant un déplacement pour **atteindre 3 sauts** quand il veut une coupe, et laisse les déplacements sans coupe à 1–2 sauts. Un autre insère `(0,0)` au milieu d'une série de 3. | règle « ≥ 3 sauts = coupe » **corroborée** |

## Sources

- pyembroidery, `DstReader.py` / `DstWriter.py` (licence MIT) — <https://github.com/EmbroidePy/pyembroidery> : encodage des bits, signature de coupe, en-tête, mode paillettes.
- libembroidery (licence zlib) — <https://github.com/Embroidermodder/libembroidery> : recoupement de la structure générale.
- Aucun code n'a été copié : seules les structures ont été recoupées.
