# Spécification du produit — Rustitch

> Document de référence fonctionnelle : ce que Rustitch doit faire, pour qui, et dans quel ordre.
> À lire avec [CONVERSION.md](CONVERSION.md) (export machine et conversion de fichiers) et [CLAUDE.md](../CLAUDE.md) (règles de code et de tests).
> Priorités : **P0** (nécessaire à la première version utilisable) · **P1** (juste après) · **P2** (plus tard) · **P3** (un jour).
> Statut : ☐ à faire · ◐ en cours · ☑ terminé.
> Révision du 2026-11-15 : le produit est recentré sur la numérisation ([ADR 0003](adr/0003-numerisation-pilotable-par-ia.md)).

---

## 0. Vision

### 0.1 Le produit en une phrase

Rustitch est une **application de numérisation de broderie pour non-professionnels** : elle transforme un dessin ou un logo en un motif brodable de vraie qualité, où **le sens et le type de remplissage donnent du relief à l'image**, et elle est conçue dès le départ pour qu'**une IA puisse la piloter** aussi bien qu'un humain.

### 0.2 Pour qui

| Utilisateur | Ce qu'il attend |
|---|---|
| **La personne qui brode chez elle** (utilisateur principal) | Possède une machine familiale. Ne connaît pas le vocabulaire du métier (sous-couche, compensation, densité) et ne veut pas régler trente paramètres. Veut un bon résultat sur tissu, et pouvoir retoucher à la souris ce qui ne lui plaît pas. |
| **Une IA** (utilisateur de premier rang) | Connectée par le Model Context Protocol (MCP). Doit pouvoir lire le motif, le voir, le modifier et vérifier son travail, sans interface graphique et sans analyser du texte libre. |
| La personne qui a téléchargé un motif | Veut broder sur sa machine Janome un fichier PES trouvé sur internet, avec les bonnes couleurs (CONVERSION). |

### 0.3 Ce qui fait la différence

Une machine familiale offre peu de couleurs par motif : **c'est le remplissage qui fait la qualité**. Un même fil brodé dans deux directions ne renvoie pas la lumière de la même façon ; les brodeurs professionnels s'en servent pour détacher deux zones d'une seule couleur (le corps et la patte d'un animal sur un logo de vêtement de sport) ou pour donner un mouvement (les rayons d'un soleil brodés chacun dans son sens).

Rustitch met ce savoir-faire à la portée d'un débutant :

- le **sens du fil** est un réglage de premier rang, visible et modifiable d'un geste, zone par zone ;
- plusieurs **types de remplissage** permettent de varier la texture avec un seul fil (§4.3) ;
- la **technique invisible** qui fait qu'une broderie ne fronce pas et ne laisse pas voir le tissu (sous-couches, compensation, densité, ordre de couture) est appliquée automatiquement (§4.4) ;
- l'**aperçu** montre le relief avant de broder (§7) ;
- une **IA** peut faire les choix artistiques à la place de l'utilisateur, ou avec lui (§6).

### 0.4 Principes de conception

1. **Le résultat sur tissu est le juge.** Une fonctionnalité de numérisation n'est terminée que lorsque ses motifs de référence ont été brodés et jugés bons (§10.2).
2. **Des objets, pas des points.** L'utilisateur et l'IA manipulent des formes et leurs réglages ; les points sont toujours recalculés par le moteur (§2).
3. **Parité humain / IA.** Tout ce qui se fait à la souris se fait par un outil MCP, et inversement : les deux passent par les mêmes commandes (§2.3, §6).
4. **Des réglages par défaut qui marchent.** Sans rien régler, le motif est brodable. Les réglages experts existent mais ne sont jamais obligatoires.
5. **Toujours retouchable.** La numérisation automatique et l'IA proposent ; l'utilisateur peut tout corriger et tout annuler.
6. **Rien n'est perdu en silence.** Toute approximation, à la numérisation comme à l'export, est signalée par un diagnostic à code stable.
7. **Fonctionne sans internet et sans IA.** L'IA est un plus, jamais une condition.
8. **Déterminisme.** Une même entrée produit exactement les mêmes points.

### 0.5 Critères de réussite

- **Parcours complet** : à partir d'un logo à aplats de 8 couleurs au plus, une personne sans connaissance du métier obtient un fichier JEF brodable sans modifier un seul réglage technique.
- **Qualité sur tissu** (motifs de référence, §10.2) : pas de jour entre deux zones voisines, pas de fronces, contours nets, détails lisibles, pas de fil sauté visible.
- **Relief** : deux zones voisines brodées avec le même fil dans deux directions différentes se distinguent à l'œil, sur l'aperçu comme sur le tissu.
- **Parité** : toute commande d'édition est exposée à la fois par l'application et par le serveur MCP ; un test automatique le vérifie.
- **IA** : à partir d'une image de référence et d'une consigne en langage courant, une IA connectée par MCP produit un motif valide et l'exporte sans intervention humaine.
- **Export** : les critères de CONVERSION §0 sont respectés.

### 0.6 Hors périmètre

- **Les photos** en entrée directe. Elles pourront passer plus tard par un pré-traitement confié à une IA (réduction des couleurs, simplification) avant la numérisation (§3.4).
- **La conversion universelle** entre tous les formats de toutes les marques (CONVERSION §0).
- Les besoins de **production industrielle** : machines multi-têtes, gestion d'atelier, chiffrage.
- Les techniques spéciales : appliqué, paillettes, broderie 3D sur mousse, dentelle.
- Les comptes, le stockage en ligne et l'application mobile.
- La promesse d'une numérisation parfaite en un clic : le résultat automatique est un bon point de départ, toujours retouchable.

---

## 1. Parcours utilisateur

Les fonctionnalités des sections suivantes existent pour servir ces parcours.

### 1.1 Numériser une image (parcours principal, P0)
1. L'utilisateur ouvre une image (logo, dessin à aplats) et indique la taille voulue ou le cadre de sa machine.
2. Il clique sur **« Numériser »** : l'application découpe l'image en zones de couleur et propose pour chacune un type de point, un sens de fil et un fil.
3. L'aperçu réaliste montre le résultat, relief compris.
4. Il retouche ce qu'il veut : tourner le sens d'un remplissage à la souris, changer de type de remplissage, couper une zone en deux pour leur donner deux directions.
5. Il exporte en JEF et brode.

### 1.2 Numériser avec une IA (P0)
1. L'utilisateur demande à son assistant IA, connecté à Rustitch par MCP, de numériser une image, avec ses envies (« les rayons du soleil chacun dans son sens, le fond en damier »).
2. L'IA crée et règle les objets avec les mêmes commandes que l'application, regarde l'aperçu, lit les diagnostics et corrige.
3. L'utilisateur voit le résultat dans l'application, le retouche ou demande des changements, puis exporte.

### 1.3 Retoucher un motif numérisé (P0)
Rouvrir un projet, modifier un sens de fil, un type de remplissage, une couleur ou l'ordre de couture, voir l'aperçu se mettre à jour, exporter à nouveau.

### 1.4 Broder un motif téléchargé (P0)
Ouvrir un fichier PES, vérifier l'aperçu, les couleurs et le cadre, l'exporter en JEF. Un fichier de points ne contient pas d'objets : on peut le voir, le placer et le convertir, pas en changer les remplissages.

---

## 2. Modèle de design (P0)

### 2.1 Objets
☐ Un **design** est une liste ordonnée d'**objets**. Chaque objet porte :
- une **forme** (contour fermé avec trous éventuels, ou tracé ouvert), en millimètres ;
- un **type de point** et ses réglages (§4), dont le **sens du fil** ;
- un **fil** (`Thread`, CONVERSION §3.1) ;
- un **identifiant stable** et un **nom** libre (« rayon 3 », « patte arrière »), pour qu'un humain et une IA parlent du même objet.

☐ L'**ordre des objets** est l'ordre de couture.
☐ Le design porte aussi : la taille finale, le cadre visé, le tissu (§4.6), l'image d'origine en calque de référence et les métadonnées.

### 2.2 Des points toujours recalculés
☐ Les points ne sont jamais stockés comme vérité : ils sont **générés** à partir des objets. Modifier un réglage régénère les points de l'objet concerné.
☐ Le résultat de la génération est un `StitchPattern` (modèle pivot, CONVERSION §1.1), seule entrée des écrivains de formats.
☐ Un fichier de points importé (PES, DST, JEF) est un design sans objets : affichable, plaçable et exportable (§1.4).

### 2.3 Commandes
☐ Toute modification d'un design est une **commande** : nommée, typée, validée avant exécution, sérialisable, annulable.
☐ L'application graphique et le serveur MCP n'ont **aucun autre moyen** de modifier un design.
☐ Annuler / rétablir illimité, commun aux actions de l'humain et de l'IA.
☐ Une commande refusée renvoie une erreur à code stable qui dit quoi corriger (valeur hors plage, objet inconnu…).

### 2.4 Fichier projet
☐ Format natif documenté et versionné : objets, réglages, fils, image d'origine. Texte lisible et déterministe (deux enregistrements du même design sont identiques).
☐ Un projet enregistré par une version reste lisible par les suivantes (migrations testées).

---

## 3. Import d'images

### 3.1 Images à aplats (P0)
☐ Formats : PNG, JPEG, BMP ; images comportant peu de couleurs (logos, dessins, pictogrammes).
☐ **Réduction des couleurs** vers un nombre choisi par l'utilisateur (défaut proposé d'après l'image, 8 au plus), avec nettoyage du lissage des bords et du bruit de compression.
☐ **Découpage en zones** d'une seule couleur, puis **vectorisation** en contours lisses.
☐ Nettoyage : fusion ou suppression des zones trop petites pour être brodées (seuil en millimètres, à la taille finale), signalées par un diagnostic.
☐ Choix du fond : la couleur de fond peut être laissée non brodée (tissu apparent).
☐ La taille finale en millimètres est demandée à l'import : tous les seuils en dépendent.

### 3.2 Dessins vectoriels (P0)
☐ Import SVG : formes remplies et tracés, couleurs, ordre d'empilement. C'est l'entrée la plus fiable, et la première développée (jalon 2).

### 3.3 Des couleurs aux fils (P1)
☐ Chaque couleur de l'image est associée au fil le plus proche (distance CIEDE2000, CONVERSION §3.2) dans un nuancier choisi.
☐ « Mes fils » : l'utilisateur déclare les fils qu'il possède ; la numérisation se limite à ceux-là.

### 3.4 Photos (hors périmètre, piste)
Une photo devra d'abord être ramenée à un dessin à aplats (peu de couleurs, formes simplifiées). Ce pré-traitement sera confié à une IA, en amont de l'import §3.1 ; il n'est pas planifié.

---

## 4. Génération de points (P0)

Le moteur transforme chaque objet en points. C'est lui qui garantit que le motif est brodable, quels que soient les choix de l'utilisateur ou de l'IA.

### 4.1 Types de points
| Type | Usage | Priorité |
|---|---|---|
| ☐ **Point droit** (running), simple ou triple | Contours, traits fins, détails | P0 |
| ☐ **Satin** | Formes étroites, bordures, lettres : points d'un bord à l'autre, très brillants | P0 |
| ☐ **Remplissage** | Formes larges, avec un sens de fil et une texture (§4.3) | P0 |

☐ Le moteur propose le type d'après la forme (étroite → satin, large → remplissage, trait → point droit) ; l'utilisateur ou l'IA peuvent le changer.
☐ Un satin trop large pour être brodé d'un seul point est découpé automatiquement ; un choix impossible (satin sur une très grande surface) est signalé avec une solution proposée.

### 4.2 Sens du fil et relief
☐ Chaque remplissage a un **sens du fil** : angle en degrés, 0° = horizontal, sens trigonométrique (Y vers le haut), de 0° inclus à 180° exclu.
☐ Une forme peut être **découpée en sous-zones** par un trait, chaque sous-zone ayant son propre sens et son propre type : c'est l'outil de base du relief à couleur égale.
☐ Le sens d'un satin suit la forme (perpendiculaire à son axe) ; il peut être incliné.
☐ Deux zones voisines de même fil ne reçoivent jamais le même sens par défaut (§5).

### 4.3 Remplissages
| Remplissage | Effet | Priorité |
|---|---|---|
| ☐ **À angle libre** (tatami) | Rangées parallèles dans le sens choisi ; le remplissage de base | P0 |
| ☐ **En contour** | Les rangées suivent le bord de la forme, vers l'intérieur | P1 |
| ☐ **Rayonnant** | Les points partent d'un centre, comme un soleil | P1 |
| ☐ **À motif** | Damier de carreaux aux sens alternés, chevrons, losanges : une texture visible avec un seul fil | P1 |
| ☐ **En spirale** | Une ligne continue du bord vers le centre | P2 |
| ☐ **Guidé par une courbe** | Le sens du fil suit une ou plusieurs lignes tracées dans la forme (fourrure, vagues, drapé) | P2 |

Pour chaque remplissage :
☐ réglages utiles exposés en clair (sens, espacement des rangées, longueur des points, taille des carreaux…), chacun avec une valeur par défaut documentée, une plage valide et une erreur claire hors plage ;
☐ les points restent à l'intérieur de la forme (à la compensation près) et respectent les longueurs minimale et maximale ;
☐ le décalage des rangées évite les alignements de piqûres qui dessineraient des lignes parasites.

### 4.4 Qualité technique automatique
Ce que l'utilisateur débutant ne règle jamais, et que le moteur fait pour lui :

☐ **Sous-couches** : points de stabilisation brodés avant la couche visible (bord, centre, zigzag, remplissage léger), choisies d'après le type et la taille de l'objet.
☐ **Compensation du tirage** : le fil tire le tissu et rétrécit les formes dans le sens des points ; les formes sont élargies d'autant.
☐ **Recouvrement** entre zones voisines, pour qu'aucun jour n'apparaisse à leur frontière.
☐ **Densité** adaptée au fil, bornée pour ne ni laisser voir le tissu ni le cartonner.
☐ **Points d'arrêt** au début et à la fin de chaque objet et avant chaque coupe.
☐ **Longueurs de points** : pas de micro-points, découpage des points trop longs, raccourcissement dans les courbes serrées des satins.
☐ **Détails trop petits** (trait plus fin qu'un fil, zone de moins d'un millimètre) : adaptés si possible, sinon signalés.

Chaque réglage automatique reste visible et modifiable dans un panneau « expert ». Les valeurs par défaut sont des valeurs de départ, **validées par les essais sur tissu** (§10.2) avant d'être déclarées fiables.

### 4.5 Ordre de couture
☐ Ordre proposé : du fond vers le premier plan, en regroupant les couleurs pour limiter les changements de fil.
☐ Points d'entrée et de sortie de chaque objet choisis pour limiter les sauts et les coupes ; trajets cachés sous les zones qui seront recouvertes.
☐ L'utilisateur et l'IA peuvent réordonner ; un ordre qui dégrade le résultat (un fond brodé par-dessus un détail) est signalé.

### 4.6 Tissu (P1)
☐ L'utilisateur choisit son tissu dans une courte liste (coton tissé, jersey extensible, éponge, polaire…) ; ce choix règle la compensation, les sous-couches et la densité à sa place.

### 4.7 Diagnostics
☐ La génération renvoie des **diagnostics** à code stable, avec l'objet concerné et un message clair : densité excessive, forme trop petite, satin trop large, trop de coupes, motif plus grand que le cadre…
☐ Statistiques : nombre de points, de couleurs, de coupes, dimensions, temps de broderie estimé, longueur de fil par couleur.

---

## 5. Numérisation automatique (P0)

Le bouton « Numériser » (§1.1) enchaîne l'import (§3) et un premier jeu de choix, sans IA et sans connexion :

☐ type de point d'après la largeur de chaque forme (§4.1) ;
☐ sens du fil d'après la forme (son grand axe), en donnant des sens **différents** à deux zones voisines du même fil pour qu'elles se détachent ;
☐ fils (§3.3), ordre de couture (§4.5), sous-couches et compensation (§4.4) ;
☐ contours : option pour border les zones d'un point droit ou d'un satin.

☐ Le résultat est un ensemble d'**objets** entièrement retouchables, jamais une simple suite de points.
☐ Le traitement est déterministe et borné en durée ; un motif trop complexe (trop de zones, détails trop fins pour la taille demandée) est signalé avec des pistes (agrandir, réduire le nombre de couleurs).

Ces choix sont des règles simples : ils donnent un motif propre et brodable. Les choix d'auteur (quel sens pour quel effet, où couper une zone) sont le domaine de l'utilisateur et de l'IA (§6).

### 5.1 Numérisation assistée par une IA, en option (P1)
☐ L'utilisateur peut demander que le bouton « Numériser » confie les choix d'auteur à une IA : après la numérisation intégrée, l'IA ajuste les sens, les types de remplissage et les découpes, par les mêmes commandes et les mêmes outils que le §6.
☐ C'est une **option, jamais une obligation** : désactivée tant que l'utilisateur ne l'a pas configurée, et l'application reste complète sans elle (§0.4, principe 7).
☐ Ce que l'IA a changé est visible, et annulable en une fois pour revenir au résultat intégré.
☐ Si l'IA est injoignable ou échoue, le résultat intégré est conservé et l'utilisateur en est informé.

---

## 6. Pilotage par une IA (P0)

Rustitch est conçu pour être utilisé par un modèle de langage via le **Model Context Protocol**. Approche : [ADR 0002](adr/0002-interface-mcp.md) et [ADR 0003](adr/0003-numerisation-pilotable-par-ia.md).

### 6.1 Ce qu'une IA doit pouvoir faire
| Besoin | Outils (noms indicatifs) |
|---|---|
| ☐ **Lire** le design sous forme structurée : objets, formes, surfaces, voisinages, réglages, fils | `describe_design`, `get_object` |
| ☐ **Voir** : obtenir une image de l'aperçu (design entier ou un objet), en mode réaliste, plan ou sens des fils (§7) | `render` |
| ☐ **Créer et modifier** : toutes les commandes du §2.3 | `import_image`, `digitize`, `split_object`, `set_stitch_type`, `set_fill`, `set_angle`, `set_thread`, `reorder`, `rename`, `undo`… |
| ☐ **Vérifier** : diagnostics et statistiques (§4.7) | `check`, `stats` |
| ☐ **Exporter et convertir** : écrire un fichier machine avec son rapport ; inspecter et convertir un fichier existant | `export`, `info`, `convert`, `list_formats`, `match_colors` |

☐ **Parité** : la liste des outils d'édition est dérivée de la liste des commandes ; un test échoue si une commande n'est pas exposée.

### 6.2 Pensé pour un modèle
☐ Entrées et sorties **structurées**, avec schémas JSON publiés ; millimètres et degrés partout ; identifiants d'objets stables.
☐ Erreurs et diagnostics à **codes stables** en `snake_case`, accompagnés de la marche à suivre (CONVERSION §1.4 pour le rapport de conversion).
☐ **Guide de numérisation** fourni par le serveur (ressource MCP) : les conventions de Rustitch et les règles du métier utiles pour bien choisir (quand préférer un satin, comment détacher deux zones de même fil, quels sens pour quel effet).
☐ Boucle **agir → voir → corriger** : chaque modification peut être suivie d'un aperçu et de diagnostics, pour que le modèle contrôle son propre travail.
☐ Aucune question posée en cours d'opération ; mêmes entrées, mêmes résultats.
☐ Descriptions d'outils courtes et sans ambiguïté ; un outil = une action.

### 6.3 Deux façons de se connecter
☐ **Sans interface** (P0) : le serveur travaille sur un fichier projet ; utilisable seul, par exemple dans une chaîne automatisée.
☐ **Dans l'application ouverte** (P1) : l'IA agit sur le design affiché ; l'utilisateur voit chaque modification en direct et peut l'annuler.

### 6.4 Garde-fous
☐ Accès aux fichiers limité à des dossiers autorisés ; taille de fichier et durée d'opération bornées ; aucun écrasement de fichier sans paramètre explicite.
☐ Tests de bout en bout : scénarios d'appels d'outils sur les fixtures, comparés à des snapshots.

---

## 7. Aperçu (P0)

L'aperçu remplace l'essai sur tissu pendant le travail ; il sert à l'utilisateur comme à l'IA.

☐ **Aperçu réaliste** : épaisseur du fil et, surtout, **éclairage dépendant du sens du fil**, pour que le relief se voie. Critère : deux zones du même fil à 0° et à 90° ont une luminosité mesurablement différente.
☐ **Mode plan** : points, sauts et coupes en couleurs distinctes.
☐ **Mode sens des fils** : hachures ou flèches indiquant la direction de chaque objet.
☐ Couleur du tissu de fond, affichage du cadre et de ses limites.
☐ Rendu **sans interface graphique**, vers une image PNG, déterministe (testable par golden files, utilisable par le serveur MCP).
☐ Statistiques affichées (§4.7).
☐ Simulation de couture animée, à vitesse réglable (P1).
☐ Relief et ombrage du fil plus poussés, texture du tissu (P2).
☐ Fiche de production imprimable : aperçu, fils dans l'ordre, cadre (P1 ; CONVERSION §4.3).

---

## 8. Application graphique (P0)

Une application de bureau, dans l'esprit des visionneuses de broderie gratuites, avec l'édition des objets en plus.

### 8.1 Première version (P0)
☐ Ouvrir une image, un SVG, un projet ou un fichier de points (PES, JEF, DST) ; enregistrer le projet.
☐ Zone de travail : zoom, déplacement, cadre affiché, image d'origine en calque (visible, estompée ou masquée).
☐ Bascule aperçu réaliste / plan / sens des fils (§7).
☐ Bouton **« Numériser »** (§5).
☐ Liste des objets dans l'ordre de couture : sélection, renommage, réordonnancement, suppression.
☐ Sur l'objet sélectionné : **poignée pour tourner le sens du fil à la souris**, choix du type de point et du remplissage dans une galerie illustrée, choix du fil.
☐ Outil pour **couper une forme** en sous-zones (§4.2).
☐ Placement du motif : déplacer, tourner, redimensionner (les points sont régénérés, la densité reste correcte).
☐ Choix du cadre et de la machine ; export (CONVERSION §4).
☐ Annuler / rétablir illimité.
☐ Diagnostics affichés sur l'objet concerné, avec la correction proposée.
☐ Vocabulaire simple, infobulles ; panneau « expert » replié par défaut.

### 8.2 Ensuite
☐ Édition des contours point par point (P1).
☐ Tracer une forme à la main par-dessus l'image (P1).
☐ Combiner plusieurs designs (P1).
☐ Panneau montrant les actions de l'IA en direct (P1, avec §6.3).
☐ Texte avec polices de broderie (P2).
☐ Mesure de distances (P2).

---

## 9. Export et conversion (P0)

☐ Un design numérisé s'exporte par la chaîne objets → points → `StitchPattern` → écrivain de format, avec les mêmes garanties et le même rapport qu'une conversion.
☐ Format de la première version : **JEF**, pour la machine d'essai (famille Janome, cadres SQ20b, RE20b et RE28b). DST est déjà disponible.
☐ Le motif est vérifié contre le cadre choisi avant l'export : jamais de troncature silencieuse.

Tout le détail (formats, couleurs, cadres, rapport, ligne de commande, recette sur machine) est dans [CONVERSION.md](CONVERSION.md).

---

## 10. Exigences transverses

### 10.1 Performance
☐ Modifier un réglage régénère l'objet et rafraîchit l'aperçu sans attente perceptible sur un motif courant (objectif : moins de 100 ms pour un objet de 10 000 points, à mesurer par benchmark).
☐ Numérisation automatique d'un logo courant en quelques secondes au plus.

### 10.2 Qualité et non-régression
☐ **Motifs de référence** dessinés pour le projet, donc versionnés et testés en CI, chacun avec son entrée, ses points attendus (golden files) et son aperçu attendu :
- mire d'angles : le même carré, même fil, brodé dans huit sens ;
- soleil à rayons, chaque rayon dans son sens ;
- silhouette d'une seule couleur découpée en sous-zones ;
- aplats voisins de couleurs différentes (jours, recouvrement) ;
- détails fins et petit texte ;
- un motif par type de remplissage.

☐ **Images réelles** (logos et dessins que l'on veut vraiment broder) : beaucoup sont protégées par le droit d'auteur et ne peuvent pas être redistribuées. Elles restent sur la machine du mainteneur, dans `tests/fixtures/private/`, hors du dépôt public ; leurs tests ne tournent qu'en local et le disent quand ils n'ont rien vérifié.
☐ **Recette sur tissu** : à chaque jalon de numérisation, les motifs de référence sont brodés sur la machine d'essai ; résultats et photos consignés dans `docs/acceptance/` (machine, fil, tissu, date). Toute valeur par défaut du moteur s'appuie sur une recette.
☐ Tests de propriétés du moteur : points dans la forme, longueurs respectées, déterminisme (CLAUDE.md §5.2).
☐ Tout motif qui a mal brodé devient une fixture de régression.

### 10.3 Plateformes et distribution
◐ Windows, macOS, Linux (bibliothèques et ligne de commande testées en CI sur les trois).
☐ Application installable sans outil de développement (installeur ou exécutable par système), Windows en premier.
☐ Licence GPL-3.0-or-later.

### 10.4 Langues
☐ Interface en français et en anglais ; noms de fils dans la langue du nuancier.
☐ Messages de diagnostic traduisibles ; les codes, eux, ne changent pas.

---

## 11. Jalons

Chaque jalon livre quelque chose d'utilisable et se clôt par une recette.

1. **Conversion PES → JEF validée sur machine** (◐ : le code est en place, voir [TODO.md](TODO.md)). Reste la recette sur la machine d'essai ([acceptance/janome.md](acceptance/janome.md)) et les corrections qui en sortiront.
2. **Moteur de points** (découpage en tâches : [jalons/jalon-2.md](jalons/jalon-2.md)) : modèle d'objets (§2.1, §2.2), import SVG (§3.2), point droit, satin et remplissage à angle libre avec découpe en sous-zones (§4.1 à §4.3), qualité technique de base (§4.4), diagnostics, aperçu réaliste en PNG (§7), commande en ligne de commande. Recette : mire d'angles et soleil brodés.
3. **Socle « prêt pour l'IA »** : commandes et annulation (§2.3), fichier projet (§2.4), serveur MCP sans interface (§6). Recette : une IA numérise le soleil à partir d'un SVG et d'une consigne.
4. **Application** : première version de l'éditeur (§8.1). Recette : le parcours §1.3 à la souris.
5. **Images à aplats et bouton « Numériser »** : import d'images (§3.1), numérisation automatique (§5), ordre de couture (§4.5). Recette : le parcours §1.1 sur trois logos, brodés.
6. **Remplissages et confort** : en contour, rayonnant, à motif (§4.3), tissus (§4.6), fils de l'utilisateur (§3.3), IA dans l'application ouverte (§6.3) et en option du bouton « Numériser » (§5.1), simulation de couture, fiche de production.
7. **Plus tard** : spirale et remplissage guidé, édition des contours, texte, autres formats (CONVERSION §2.2), photos (§3.4).

---

## 12. Questions ouvertes

À trancher avant le jalon concerné ; chaque décision d'architecture fera l'objet d'un ADR.

| Question | Pour le jalon |
|---|---|
| Modèle exact de la machine d'essai (« eXpressive », famille Janome) et version de son logiciel interne | 1 |
| Ordre des jalons 3, 4 et 5 : l'IA avant l'application, ou l'inverse ? | 3 |
| Format et extension du fichier projet | 3 |
| Option IA du bouton « Numériser » (§5.1) : quel service, comment l'utilisateur le configure (clé d'accès, coût) | 6 |
| Bibliothèque d'interface graphique | 4 |
| Méthode de vectorisation des images (bibliothèque existante ou code propre) | 5 |
| Mécanisme de connexion de l'IA à l'application ouverte | 6 |
| Nuanciers de fils à intégrer en premier | 6 |
