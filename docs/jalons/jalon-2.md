# Jalon 2 — Moteur de points : découpage en tâches

> Périmètre : [FEATURES §11](../FEATURES.md), jalon 2. Règles de code et de tests : [CLAUDE.md](../../CLAUDE.md).
> Statut du document : proposé le 2026-11-15, à valider. Statut des tâches : ☐ à faire · ◐ en cours · ☑ terminé.

## 1. But du jalon

À la fin du jalon, cette commande donne un fichier brodable et son aperçu :

```bash
emb digitize soleil.svg soleil.jef --width 100mm --preview soleil.png
```

à partir d'un dessin vectoriel dont chaque forme porte son type de point et son sens de fil. Le jalon se clôt quand la **mire d'angles** et le **soleil** ont été brodés sur la machine d'essai et jugés bons.

### Dans le jalon
- Formes, découpe en sous-zones, décalages de contours (`emb-geometry`).
- Design à objets, sans commandes ni annulation (FEATURES §2.1, §2.2).
- Point droit, satin le long d'un tracé, remplissage à angle libre (FEATURES §4.1 à §4.3).
- Sous-couches, compensation du tirage, points d'arrêt, longueurs de points (FEATURES §4.4).
- Diagnostics et statistiques (FEATURES §4.7).
- Import SVG (FEATURES §3.2).
- Aperçu en PNG, modes plan et réaliste (FEATURES §7).
- Ligne de commande, motifs de référence, recette sur tissu.

### Hors du jalon
| Sujet | Où |
|---|---|
| Commandes, annulation, fichier projet, serveur MCP | jalon 3 |
| Application graphique | jalon 4 |
| Images en pixels, bouton « Numériser », choix automatique du type de point, satin déduit d'une forme fermée étroite, ordre de couture optimisé, recouvrement automatique entre zones voisines | jalon 5 |
| Remplissages en contour, rayonnant, à motif ; tissus | jalon 6 |

## 2. Règles communes à toutes les tâches

- **Une tâche = une pull request = un objectif** (CLAUDE.md §1.6). Si une tâche se révèle trop grosse, la recouper ici avant de coder.
- Les tests sont écrits avant ou avec le code ; les **critères d'acceptation** ci-dessous sont le minimum, pas la liste complète des tests.
- Déterminisme au bit près ; aucune panique ; erreurs typées ; millimètres en `Mm`.
- **Aucune valeur métier inventée** : toute valeur par défaut vient du tableau de la tâche J2-02, avec sa source.
- Un crate n'est créé que par la première tâche qui en a besoin.
- Chaque tâche met à jour `CHANGELOG.md` et coche sa case ici.

## 3. Vue d'ensemble

| Lot | Tâches | Ce qu'on obtient |
|---|---|---|
| A. Fondations | J2-01 à J2-03 | Les décisions d'architecture et les valeurs par défaut, écrites et validées |
| B. Voir | J2-04, J2-05 | `emb render` : un aperçu PNG de n'importe quel fichier de points, y compris les PES et JEF existants |
| C. Géométrie | J2-06 à J2-09 | Formes fiables : décalage, hachurage, découpe |
| D. Modèle | J2-10 | Le design à objets |
| E. Points | J2-11 à J2-18 | Les générateurs, la qualité technique, l'assemblage, les diagnostics |
| F. Entrée et sortie | J2-19 à J2-21 | SVG → JEF en une commande |
| G. Références et recette | J2-22 à J2-24 | Motifs de référence, mesures, essai sur tissu |

Ordre conseillé : A, puis B (on voit ce qu'on fait dès le début), puis C et D, puis E dans l'ordre des numéros, F, G. Les tâches d'un même lot qui ne dépendent pas l'une de l'autre peuvent avancer en parallèle (colonne « Dépend de »).

Tâches à risque, à traiter avec le plus de soin : **J2-07** (décalage de contours), **J2-13** (remplissage des formes à creux et à trous), **J2-14** (satin), **J2-05** (rendu identique sur les trois systèmes).

---

## 4. Lot A — Fondations

### ☐ J2-01 — ADR 0004 : architecture du moteur
**Objectif** : fixer, avant tout code, ce que les tâches suivantes ne doivent pas décider chacune de leur côté.
**Contenu** (document seul, aucun code) :
- crates créés et leur graphe de dépendances. Proposition de départ : `emb-core` ← `emb-geometry` ← `emb-stitch` ← `emb-project` ; `emb-import` produit des formes et dépend de `emb-geometry` ; `emb-render` ne dépend que de `emb-core` ;
- où vivent `Shape`, `Object`, `Design`, les paramètres de points ;
- bibliothèques externes, chacune justifiée (maintenance, licence, taille) et comparée à au moins une alternative : courbes et opérations sur polygones (décalage, intersection), lecture du SVG, tracé d'image et écriture PNG ;
- comment une forme SVG porte son type de point et son sens de fil (attributs dédiés, ou identifiants et fichier d'accompagnement) ;
- tolérances nommées (aplatissement des courbes, appartenance à une forme).

**Acceptation** : ADR relu et accepté par le mainteneur ; `cargo deny` accepte les licences retenues.
**Dépend de** : —

### ☐ J2-02 — Paramètres de points : valeurs par défaut et sources
**Objectif** : aucune valeur métier dans le code sans origine connue.
**Contenu** : `docs/points/parametres.md`, un tableau par type de point (point droit, satin, remplissage, sous-couches, compensation, points d'arrêt, seuil saut/coupe) : nom, unité, valeur par défaut, plage valide, **source** (documentation d'un fabricant de fils ou de machines, logiciel libre de référence avec sa licence, essai sur tissu), statut « à valider sur tissu » ou « validé le … ».
**Acceptation** : chaque paramètre utilisé par les tâches du lot E y figure ; aucune valeur sans source ; les valeurs en désaccord entre sources sont signalées, pas tranchées en silence.
**Dépend de** : —

### ☐ J2-03 — Garde-fou : règles de dépendances vérifiées par un test
**Objectif** : qu'une modification ne puisse pas violer l'architecture sans faire échouer la CI ([TODO.md](../TODO.md) §5).
**Contenu** : test lisant `cargo metadata` : graphe conforme à l'ADR 0004, `emb-core` sans dépendance interne, aucune bibliothèque graphique hors de `emb-app`.
**Acceptation** : le test échoue si l'on ajoute une dépendance interdite (vérifié par un cas de test sur un graphe fabriqué).
**Dépend de** : J2-01

---

## 5. Lot B — Voir

L'aperçu vient tôt : c'est l'outil de mise au point de tout le reste, pour un humain comme pour une IA, et il sert tout de suite à contrôler les conversions existantes.

### ☐ J2-04 — `emb-render` : aperçu « plan » en PNG et commande `emb render`
**Objectif** : obtenir une image d'un `StitchPattern`.
**Contenu** : points tracés en traits de la couleur du fil ; sauts et coupes dans des styles distincts ; cadre optionnel ; échelle en pixels par millimètre ; fond uni. Commande `emb render fichier image.png`, pour tout format lisible (PES, JEF, DST).
**Acceptation** :
- la fixture asymétrique « F » sort à l'endroit (le passage de Y vers le haut à l'image est testé) ;
- images de référence identiques sur les trois systèmes de la CI ; si c'est impossible, comparaison avec une tolérance documentée et justifiée dans la PR ;
- même entrée, même image, octet pour octet sur une même machine ;
- aucune dépendance graphique à fenêtre.

**Dépend de** : J2-01

### ☐ J2-05 — Aperçu réaliste : épaisseur et éclairage selon le sens du fil
**Objectif** : voir le relief avant de broder (FEATURES §7).
**Contenu** : chaque point est un brin épais ; sa luminosité dépend de l'angle entre le point et une source de lumière fixe ; option `--mode plan|realiste`, couleur du tissu.
**Acceptation** :
- deux carrés du même fil, remplis à 0° et à 90°, ont une luminosité moyenne qui diffère d'au moins un seuil fixé dans le test ;
- la luminosité d'un point ne dépend pas de son sens de parcours (brodé de A vers B ou de B vers A) ;
- images de référence comme en J2-04.

**Dépend de** : J2-04

---

## 6. Lot C — Géométrie (`emb-geometry`)

### ☐ J2-06 — Formes et tracés
**Contenu** : tracé ouvert, forme fermée avec trous ; validation (fermée, non dégénérée, sans auto-intersection, sens de parcours normalisé) ; surface, boîte englobante, appartenance d'un point ; aplatissement des courbes avec tolérance ; translation, rotation, mise à l'échelle.
**Acceptation** : tests de propriétés (surface invariante par rotation et translation ; un point intérieur le reste après transformation) ; cas dégénérés refusés par une erreur typée (forme vide, surface nulle, trou hors de la forme).
**Dépend de** : J2-01

### ☐ J2-07 — Décalage de contours (vers l'intérieur et l'extérieur)
**Objectif** : la brique de la compensation, des sous-couches en bordure et du débord.
**Contenu** : décalage d'une distance donnée ; une forme qui disparaît en rétrécissant donne un résultat vide, pas une erreur ; une forme peut se scinder en plusieurs ; les trous sont décalés dans l'autre sens.
**Acceptation** : propriétés : la forme élargie contient l'originale, la forme rétrécie y est contenue ; aucun point du résultat à moins de la distance demandée du contour d'origine (à la tolérance près) ; cas tests : sablier qui se scinde, anneau dont le trou se referme.
**Dépend de** : J2-06

### ☐ J2-08 — Hachurage : rangées parallèles dans une forme
**Contenu** : pour un sens et un espacement donnés, les segments de chaque rangée à l'intérieur de la forme, triés ; convention d'angle de FEATURES §4.2.
**Acceptation** : propriétés : tout segment est dans la forme ; la longueur totale multipliée par l'espacement approche la surface ; un angle hors de l'intervalle de FEATURES §4.2 est refusé ; angles 0°, 90° et quelconques ; rangées tangentes à un sommet ou confondues avec un bord traitées sans doublon ni trou.
**Dépend de** : J2-06

### ☐ J2-09 — Découpe d'une forme par un trait
**Objectif** : l'outil de base du relief à couleur égale (FEATURES §4.2).
**Acceptation** : la somme des surfaces des sous-formes égale la surface d'origine ; les sous-formes ne se chevauchent pas ; un trait qui ne traverse pas la forme la laisse intacte et le dit ; trait passant par un trou.
**Dépend de** : J2-06

---

## 7. Lot D — Modèle

### ☐ J2-10 — Design à objets
**Contenu** (FEATURES §2.1) : `Design` (liste ordonnée d'objets, taille, cadre, couleur du tissu, métadonnées) ; `Object` (identifiant stable, nom, forme ou tracé, type de point avec ses paramètres, fil) ; validation des paramètres contre les plages de J2-02, avec erreur typée à code stable ; découpe d'un objet en deux objets (J2-09) qui héritent de ses réglages.
**Hors tâche** : commandes, annulation, fichier projet (jalon 3). Le modèle est conçu pour les recevoir sans changer de forme.
**Acceptation** : un paramètre hors plage est refusé avec le nom du paramètre et la plage ; les identifiants restent stables quand on réordonne ; états invalides non représentables (un remplissage sur un tracé ouvert ne se construit pas).
**Dépend de** : J2-01, J2-02, J2-06, J2-09

---

## 8. Lot E — Points (`emb-stitch`)

Propriétés exigées de **tous** les générateurs (CLAUDE.md §5.2) : longueurs de points dans les bornes, points dans la forme à la tolérance près, déterminisme, aucune panique sur une forme valide quelconque.

### ☐ J2-11 — Point droit, simple et triple
**Contenu** : points régulièrement espacés le long d'un tracé, sommets conservés, dernier point sur la fin du tracé ; variante triple.
**Acceptation** : aucun point plus long que le maximum ni plus court que le minimum (sauf tracé plus court que le minimum, signalé) ; tous les points sur le tracé à la tolérance près.
**Dépend de** : J2-06

### ☐ J2-12 — Remplissage à angle libre, formes simples
**Contenu** : formes dont chaque rangée est d'un seul tenant ; aller-retour d'une rangée à la suivante ; longueur de point ; décalage des piqûres d'une rangée à l'autre pour éviter les alignements.
**Acceptation** : propriétés communes ; espacement des rangées respecté ; les extrémités des rangées sont sur le contour ; mire à 0°, 22,5°, 45°, 90°, 135° comparée à des snapshots ; rendu J2-05 visiblement différent d'un angle à l'autre.
**Dépend de** : J2-08

### ☐ J2-13 — Remplissage à angle libre, formes à creux et à trous
**Objectif** : remplir n'importe quelle forme valide en un seul parcours.
**Contenu** : découpage en régions d'un seul tenant, ordre de parcours, trajets de liaison **à l'intérieur de la forme**, sous les rangées à venir.
**Acceptation** : aucun point ni trajet hors de la forme ou au-dessus d'un trou ; toute la surface est couverte (chaque segment de J2-08 est brodé une fois) ; aucun saut ni coupe à l'intérieur d'un objet ; fixtures : anneau, « C », étoile, forme à deux trous.
**Dépend de** : J2-12

### ☐ J2-14 — Satin le long d'un tracé
**Contenu** : satin défini par deux bords ; construction à partir d'un tracé et d'une largeur (contours, bordures) ; espacement ; découpage automatique au-delà de la largeur maximale ; en dessous de la largeur minimale, repli sur un point droit avec diagnostic ; points raccourcis à l'intérieur des courbes serrées.
**Hors tâche** : satin déduit automatiquement d'une forme fermée étroite (jalon 5).
**Acceptation** : chaque point va d'un bord à l'autre ; aucun point plus long que le maximum ; virage serré sans accumulation de piqûres au même endroit (densité bornée, testée) ; tracé fermé raccordé sans trou.
**Dépend de** : J2-06, J2-07

### ☐ J2-15 — Sous-couches
**Contenu** : pour un remplissage, tour de bordure en retrait et remplissage léger dans un autre sens ; pour un satin, ligne centrale, bordures ou zigzag selon la largeur ; choix automatique d'après le type et la taille, modifiable.
**Acceptation** : la sous-couche reste à l'intérieur de la couche visible ; elle est brodée avant ; un objet trop petit pour en recevoir n'en a pas, sans erreur.
**Dépend de** : J2-13, J2-14, J2-07

### ☐ J2-16 — Compensation du tirage et débord
**Contenu** : élargissement de la forme dans le sens des points (valeur de J2-02) ; débord réglable par objet, pour passer sous l'objet voisin.
**Hors tâche** : débord calculé automatiquement d'après les voisins (jalon 5).
**Acceptation** : la largeur brodée dans le sens des points augmente de la valeur demandée, pas dans l'autre sens ; compensation nulle = forme d'origine au bit près.
**Dépend de** : J2-07, J2-13, J2-14

### ☐ J2-17 — Assemblage : du design au `StitchPattern`
**Contenu** : objets dans l'ordre ; un bloc de couleur par suite d'objets du même fil ; déplacement entre objets par saut ou par coupe selon la distance ; points d'arrêt au début et à la fin de chaque objet et avant chaque coupe ; suppression des micro-points ; fin.
**Acceptation** : le `StitchPattern` produit respecte les invariants de `emb-core` ; écrit en JEF puis relu, il est identique à sa version quantifiée (CONVERSION §1.2) ; deux objets voisins du même fil ne provoquent pas d'arrêt machine.
**Note** : le seuil saut/coupe pour le JEF dépend de la recette du jalon 1 (fichier `04` : la machine coupe-t-elle d'elle-même ?).
**Dépend de** : J2-10 à J2-16

### ☐ J2-18 — Diagnostics et statistiques
**Contenu** (FEATURES §4.7) : diagnostics typés à code stable, avec l'objet concerné : densité excessive, forme trop petite, satin découpé ou replié, motif plus grand que le cadre, nombre de coupes élevé ; statistiques : points, couleurs, coupes, dimensions, temps estimé, longueur de fil par couleur (en étendant `PatternStats`).
**Acceptation** : chaque code a un test qui le déclenche et un test qui ne le déclenche pas ; les codes sont listés dans la documentation ; rien n'est seulement écrit dans les logs.
**Dépend de** : J2-17

---

## 9. Lot F — Entrée et sortie

### ☐ J2-19 — Import SVG : géométrie (`emb-import`)
**Contenu** : rectangles, cercles, ellipses, polygones, lignes, chemins avec courbes ; groupes et transformations ; unités et `viewBox` vers les millimètres ; inversion de l'axe Y ; règle de remplissage (trous).
**Acceptation** : un « F » dessiné en SVG sort à l'endroit ; un carré de 10 mm déclaré en mm, en pouces et en pixels mesure 10 mm ; ce qui n'est pas pris en charge (dégradés, texte, masques, images incluses) donne un avertissement nommant l'élément, jamais un silence ; aucune panique sur une entrée quelconque (test de propriétés).
**Dépend de** : J2-06

### ☐ J2-20 — Import SVG : des formes aux objets
**Contenu** : forme remplie → remplissage ; trait → point droit ou satin selon son épaisseur ; couleur → fil ; ordre du document → ordre de couture ; type de point et sens de fil lus selon la convention de l'ADR 0004 ; mise à la taille demandée.
**Acceptation** : la mire d'angles et le soleil sont entièrement décrits par leur fichier SVG ; un paramètre invalide dans le SVG est refusé avec l'identifiant de la forme ; sans indication, valeurs par défaut de J2-02.
**Dépend de** : J2-10, J2-19

### ☐ J2-21 — Commande `emb digitize`
**Contenu** : SVG → fichier machine, avec `--width`/`--height`, `--hoop`, `--preview`, format déduit de l'extension ; diagnostics sur la sortie d'erreur ; mêmes codes de sortie que `emb convert` (0, 3, 1, 2). La logique est dans les bibliothèques (service partagé, comme `emb_formats::convert`), pas dans la CLI.
**Acceptation** : tests de bout en bout `assert_cmd` : SVG valide → JEF relisible et PNG ; motif plus grand que le cadre → échec explicite ; SVG avec élément non pris en charge → code 3.
**Dépend de** : J2-05, J2-17, J2-18, J2-20

---

## 10. Lot G — Références et recette

### ☐ J2-22 — Motifs de référence
**Contenu** : SVG **dessinés pour le projet**, donc versionnés et testés en CI : mire d'angles (le même carré dans huit sens), soleil (disque, rayons chacun dans son sens, contour en satin), anneau et formes à trous, aplats voisins de deux couleurs, détails fins. Pour chacun : snapshot des points et des statistiques, images de référence plan et réaliste.
**Acceptation** : toute modification d'un générateur qui change un motif fait échouer un test ; la mise à jour d'une snapshot suit CLAUDE.md §5.3.
**Note** : les images réelles du mainteneur, non redistribuables, restent dans `tests/fixtures/private/` ; elles serviront au jalon 5 (images en pixels).
**Dépend de** : J2-21

### ☐ J2-23 — Mesures de performance
**Contenu** : benchmarks `criterion` : remplissage d'une grande forme, régénération d'un objet de 10 000 points, rendu réaliste (FEATURES §10.1).
**Acceptation** : valeurs de référence consignées ; objectif de 100 ms pour l'objet de 10 000 points atteint, ou écart expliqué et entrée ajoutée à `TODO.md`.
**Dépend de** : J2-17, J2-05

### ☐ J2-24 — Recette sur tissu (clôture du jalon)
**Contenu** : `docs/acceptance/jalon-2.md` (checklist sur le modèle de la recette Janome) et générateur des fichiers pour la clé USB ; le mainteneur brode la mire d'angles et le soleil, sur son tissu et avec son fil habituels.
**À observer** : relief visible entre deux sens ; jours entre zones ; fronces ; netteté des bords ; fils sautés ; écart de taille entre le dessin et la broderie (mesure de la compensation).
**Acceptation** : résultats et photos consignés dans `docs/acceptance/resultats/` (machine, fil, tissu, date) ; valeurs de J2-02 corrigées d'après les mesures et marquées « validé » ; chaque défaut constaté devient une fixture de régression ou une entrée de `TODO.md`.
**Dépend de** : J2-22, et recette du jalon 1 ([acceptance/janome.md](../acceptance/janome.md))

---

## 11. Ce qui peut bloquer

| Point | Effet | Parade |
|---|---|---|
| Recette du jalon 1 non faite | J2-24 impossible ; seuil saut/coupe de J2-17 provisoire | Le reste du jalon avance sans elle |
| Rendu différent d'un système à l'autre | Images de référence instables en CI | Tolérance documentée, décidée en J2-04 |
| Décalage de contours fragile sur des formes réelles | Sous-couches et compensation fausses | Bibliothèque éprouvée choisie en J2-01 ; fixtures de formes difficiles en J2-07 |
| Valeurs par défaut sans source fiable | Motifs qui froncent ou laissent voir le tissu | J2-02 avant tout générateur ; J2-24 tranche |
