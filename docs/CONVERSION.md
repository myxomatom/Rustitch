# Spécification de la conversion de formats

> Document de référence pour **l'export machine et la conversion de fichiers de broderie**, dernier maillon de Rustitch. La vision du produit, la numérisation, l'application et le pilotage par une IA sont décrits dans [FEATURES.md](FEATURES.md) ; les règles de code et de tests dans [CLAUDE.md](../CLAUDE.md).
> Les numéros des sections 1 à 5 sont stables : le code, les ADR et `TODO.md` y renvoient (« CONVERSION §3.4 »).
> Priorités : **P0** (nécessaire à la première version) → **P3** (plus tard). Statut : ☐ à faire · ◐ en cours · ☑ terminé.

---

## 0. Rôle et priorité

La conversion sert deux usages :

1. **exporter** vers la machine les motifs numérisés dans Rustitch (FEATURES §9) ;
2. **convertir un motif existant**, le cas prioritaire étant un fichier **PES** trouvé sur internet, à broder sur une machine de la famille **Janome** (format **JEF**).

Le trajet **PES → JEF** doit être irréprochable avant tout élargissement. La conversion entre tous les formats de toutes les marques n'est **pas** un objectif en soi : les autres formats viendront un par un, quand un besoin réel se présentera (§2.2).

### Ce qu'une conversion peut conserver

Aucun format ne stocke tout. « Fidèle » signifie donc : tout ce que le format cible peut porter est conservé, et **tout le reste est signalé**, jamais perdu en silence.

| Trajet | Géométrie | Blocs de couleur | Couleurs des fils | Coupes |
|---|---|---|---|---|
| PES → JEF | ≤ 0,1 mm | conservés | index de la palette Janome, plus le fil réel (couleur, référence numérique, nom) dans le bloc `ENHANCE` ; la marque est perdue | non stockées par le JEF : comptées et signalées ; comportement de la machine à vérifier (§4.5) |
| DST → JEF | ≤ 0,1 mm | conservés | absentes de la source : couleurs arbitraires, signalées | déduites des séquences de sauts de la source, puis comme ci-dessus |
| PES → DST, JEF → DST | ≤ 0,1 mm | conservés (arrêts) | perdues (le DST n'en stocke pas), signalées | encodées par des séquences de sauts |

### Critères de réussite globaux
- Une conversion A → B → A ne dérive pas : écart de position ≤ 0,1 mm sur chaque point, sans accumulation d'erreur.
- Aucun bloc de couleur n'est fusionné, supprimé ou réordonné.
- Deux fils distincts dans le fichier source restent distincts dans le fichier cible, sauf impossibilité signalée explicitement.
- Chaque conversion produit un **rapport de conversion** listant toute perte ou approximation.
- Un JEF écrit par Rustitch est accepté par la machine d'essai et brodé comme l'original (recette, §4.5).

---

## 1. Moteur de conversion (P0)

### 1.1 Modèle pivot
☑ Toutes les conversions passent par un modèle interne unique (`StitchPattern`) : N lecteurs + N écrivains, jamais de convertisseur direct format à format.

Le modèle pivot doit conserver **plus d'informations que n'importe quel format** pour ne rien perdre en transit :
- coordonnées absolues en mm (`f64`), jamais en deltas ;
- type de chaque point : `Normal`, `Jump`, `Trim`, `ColorChange`, `Stop`, `SequinEject`, `End` ;
- liste ordonnée des blocs de couleur, chacun associé à un `Thread` complet (voir §3.1) ;
- métadonnées : nom du design, auteur, commentaires, cadre (hoop) d'origine, date ;
- une table d'extensions `format → données brutes` pour les informations propres à un format (permet un aller-retour sans perte dans le même format).

### 1.2 Quantification sans dérive
☑ La conversion vers les unités machine (généralement 0,1 mm) se fait en **arrondissant les positions absolues**, puis en calculant les deltas entre positions arrondies. Interdiction d'arrondir les deltas successifs (cela accumule l'erreur).
☑ Test de propriété : sur un motif de 100 000 points aléatoires, l'écart maximal entre position source et position exportée reste ≤ 0,05 mm.

### 1.3 Contraintes de chaque format
☑ Chaque écrivain déclare ses capacités dans une structure `FormatCapabilities` :
- longueur maximale de déplacement par enregistrement ;
- résolution des coordonnées ;
- support natif des coupes (trim), des arrêts (stop), des paillettes ;
- modèle de couleur : aucune couleur / index de palette fixe / RGB / référence catalogue ;
- nombre maximal de couleurs et de points ;
- tailles de cadre supportées.

◐ Avant écriture, une passe de **normalisation** adapte le motif aux capacités :
- découpage des points trop longs en sauts intermédiaires (répartis régulièrement, jamais un reste minuscule en fin) ;
- traduction des coupes quand le format ne les supporte pas nativement (stratégie configurable, documentée par format) ;
- suppression optionnelle des micro-points (< seuil paramétrable), désactivée par défaut pour la fidélité.

### 1.4 Rapport de conversion
☐ Chaque conversion produit un rapport (texte, JSON et HTML) contenant :
- statistiques source / cible : nombre de points, sauts, coupes, changements de couleur, dimensions ;
- écart géométrique maximal et moyen ;
- table de correspondance des couleurs avec l'écart perceptuel ΔE2000 de chaque fil ;
- avertissements : fils fusionnés, couleurs hors tolérance, informations non transposables (paillettes, noms de fils…) ;
- cadre choisi.

### 1.5 Robustesse de lecture
◐ Lecture tolérante : un fichier légèrement corrompu ou tronqué est lu jusqu'à la dernière donnée valide, avec un avertissement, sans jamais paniquer.
◐ Détection automatique du format par signature binaire, l'extension n'étant qu'un indice.
☐ Gestion des variantes et versions d'un même format (voir §2).

---

## 2. Formats

Légende : **L** lecture · **É** écriture.

### 2.1 P0 — Formats de la première version
| Marque | Format | L | É | Remarques |
|---|---|---|---|---|
| Janome / Elna / Kenmore | **.jef** | ◐ | ◐ | Format de la machine d'essai. Couleurs = index dans la palette Janome, fils réels dans le bloc `ENHANCE`. Voir §4. |
| Brother / Baby Lock / Bernina deco | **.pes** | ◐ | — | Format le plus répandu sur internet. Versions 1 à 10 lues ; les points sont dans la section PEC. |
| Tajima | .dst | ◐ | ◐ | Déjà en place. Pas d'information de couleur ; coupes encodées par séquences de sauts. |

### 2.2 Plus tard — sur besoin réel
Aucun de ces formats n'est planifié. Chacun sera ajouté quand un besoin réel se présentera, avec sa fiche et son corpus (§2.5).

- **Écriture PES** (au minimum v1 pour la compatibilité et une version récente avec fils RGB) ; lecture et écriture des `.pec` seuls (le décodeur PEC existe déjà).
- **JEF+** (Janome) : voir §4.4.
- Autres marques : .exp (Melco / Bernina), .vp3 (Husqvarna Viking / Pfaff), .xxx (Singer), .hus, .vip, .shv, .pcs, .pcd, .pcm, .pcq, .sew, .jpx, .phc, .phb, .dsb, .dsz, .tbf, .u01 (Barudan), .10o, .100 (Toyota), .tap (Happy), .z00 / .zsk (ZSK), .cnd, .csd.
- Fichiers de couleurs annexes : `.inf`, `.edr`, `.rgb`, `.col`.

### 2.3 P2 — Formats d'échange et utilitaires
☐ Export SVG (tracé des points) et PDF (fiche de production avec couleurs et ordre de couture). L'aperçu PNG relève de FEATURES §7.
☐ Import/export CSV et JSON (format texte lisible, utile pour le débogage et les tests).


### 2.4 Hors périmètre (sauf décision contraire)
- Formats propriétaires chiffrés (ex. `.art` Bernina) : à étudier uniquement si une documentation légale existe. Ne jamais contourner un chiffrement.

### 2.5 Sources de documentation
- Chaque format possède sa fiche `docs/formats/<format>.md` : structure de l'en-tête, encodage des points, commandes, limites, variantes connues, sources.
- Sources de référence à croiser : bibliothèques open source **pyembroidery** et **libembroidery** (respecter leurs licences : s'en inspirer pour comprendre la structure, ne pas copier de code incompatible), documentations des fabricants, et surtout **fichiers réels** produits par les logiciels officiels.
- Toute affirmation sur un format non confirmée par au moins un fichier réel est marquée « à vérifier » dans la fiche.

---

## 3. Gestion des couleurs et des fils (P0)

La fidélité des couleurs est le point le plus délicat de la conversion, car les formats utilisent des modèles incompatibles : certains ne stockent aucune couleur (DST, EXP), d'autres un index dans une palette fixe du fabricant (JEF, PEC), d'autres du RGB avec références catalogue (VP3, PES récent).

### 3.1 Modèle de fil
☑ Structure `Thread` :
- couleur sRGB ;
- nom ;
- marque et nuancier (Madeira, Isacord, Robison-Anton, Sulky, Gunold, Janome, Brother…) ;
- référence catalogue ;
- poids/type de fil (optionnel).

### 3.2 Distance perceptuelle
☑ Toute comparaison de couleurs utilise **CIEDE2000 (ΔE00)** dans l'espace CIELAB (illuminant D65), **jamais une distance euclidienne en RGB**, qui distingue mal les couleurs proches.
☑ Conversion sRGB → linéaire → XYZ → Lab testée contre des valeurs de référence publiées (jeu de test officiel de Sharma pour CIEDE2000).

### 3.3 Nuanciers
☐ Nuanciers intégrés et versionnés : palette Janome, palette Brother, et principaux nuanciers commerciaux.
☐ Tables de correspondance entre nuanciers (ex. Madeira ↔ Janome). Quand la référence catalogue source est connue, elle prime sur la couleur RGB pour choisir le fil cible.
☐ Nuanciers importables/éditables par l'utilisateur (son stock de fils réel).

### 3.4 Algorithme de correspondance des couleurs
Trois stratégies, sélectionnables, la stratégie **fidèle** étant celle par défaut :

1. ☑ **Plus proche voisin** (simple) : chaque fil source → entrée de palette au ΔE minimal. Rapide, mais peut attribuer le même index à deux fils proches : réservé au mode « aperçu rapide ».
2. ☑ **Affectation optimale injective** (par défaut) : on cherche l'affectation des fils source distincts vers des entrées de palette **distinctes** qui minimise la somme des ΔE (algorithme hongrois / Kuhn-Munkres). Ainsi, deux fils proches mais différents dans la source restent deux fils différents dans la cible.
3. ☐ **Préservation du contraste** (option) : en plus de l'injectivité, pénaliser les affectations qui inversent l'ordre de clarté (L*) ou de teinte entre deux fils proches, pour que le dégradé ou le relief voulu reste lisible.

Règles communes :
☑ Identité des fils : deux blocs utilisent « le même fil » s'ils ont la même couleur **et** la même référence ; sinon ce sont des fils distincts, même à ΔE < 1.
☑ Si le nombre de fils distincts dépasse la taille de la palette, fusionner en priorité les paires au plus faible ΔE et le signaler dans le rapport.
◐ Seuils d'alerte configurables (par défaut : avertissement si ΔE00 > 3, erreur bloquante optionnelle si > 8).
☐ Surcharge manuelle : l'utilisateur peut forcer l'affectation de n'importe quel fil ; ses choix sont mémorisés par design.
☑ Déterminisme : à égalité de coût, départage par index de palette croissant.

### 3.5 Formats sans couleur
☐ À la lecture d'un DST/EXP, utiliser le fichier de couleurs annexe s'il existe, sinon une palette par défaut contrastée, et le signaler.
☐ À l'écriture vers un format sans couleur, générer en option le fichier annexe et la fiche de production.

---

## 4. Conversion vers JEF (P0 — soin particulier)

### 4.1 Contraintes du format JEF à respecter
☑ Coordonnées en 0,1 mm, deltas signés sur un octet ; tout déplacement plus long est découpé.
◐ Commandes de contrôle (changement de couleur, déplacement, fin) encodées selon la fiche `docs/formats/jef.md`, établie et vérifiée sur des fichiers produits par les logiciels Janome.
☑ Axe Y : vérifier le sens par rapport au modèle pivot avec une fixture asymétrique (ex. la lettre « F ») — un motif inversé doit faire échouer un test.
◐ En-tête cohérent : décalage des données, date, nombre de couleurs, nombre de points, code du cadre et emprises (extents) calculées à partir du motif réel.
☑ Table des couleurs : un index de palette Janome par bloc, dans l'ordre exact des blocs, plus le champ de type de fil associé.

### 4.2 Choix du cadre (hoop)
◐ Sélection automatique du plus petit cadre Janome contenant le motif, centré.
◐ Choix manuel ou par modèle de machine cible (profils de machines Janome avec leurs cadres disponibles).
☑ Erreur explicite si le motif dépasse le plus grand cadre de la machine ciblée (jamais de troncature silencieuse).

### 4.3 Fidélité des couleurs vers JEF
Le JEF ne stocke qu'un index dans la palette Janome : c'est là que les couleurs proches risquent d'être confondues.

☑ Utiliser par défaut l'**affectation optimale injective** (§3.4) sur la palette Janome.
☑ Si la source contient des références de fils Janome, les reprendre telles quelles (correspondance exacte, ΔE ignoré).
☐ Si la source contient des références d'autres marques, passer par les tables de correspondance de nuanciers avant de recourir au RGB.
☑ Ne **jamais** fusionner deux blocs consécutifs, même s'ils obtiennent le même index : chaque changement de couleur source reste un arrêt machine (l'utilisateur peut avoir changé de fil volontairement).
☐ Option « couleurs exactes » : générer à côté du JEF une **fiche de fils** (PDF/HTML/CSV) indiquant pour chaque bloc l'index Janome affiché par la machine **et** le fil réel d'origine (marque, référence, nom, échantillon de couleur), afin que la brodeuse enfile les vraies couleurs.
☐ Aperçu côte à côte dans l'interface : couleurs d'origine / couleurs Janome attribuées, avec mise en évidence des paires de fils proches et de leur ΔE.

### 4.4 JEF+ (plus tard)
Non planifié tant que le JEF suffit à la machine d'essai ; les exigences ci-dessous s'appliqueront le jour où il le sera.

☐ **Phase d'investigation obligatoire avant implémentation** : le JEF+ est un format étendu peu documenté publiquement.
- Constituer un corpus de fichiers JEF+ produits par les logiciels Janome (avec, si possible, le JEF équivalent du même design).
- Analyser les différences avec le JEF (données supplémentaires, informations de fils, objets éditables…).
- Documenter les résultats et les incertitudes dans `docs/formats/jef-plus.md` et consigner la décision dans un ADR.
- Rétro-ingénierie uniquement par observation de fichiers (approche « salle blanche »), sans désassemblage de logiciel propriétaire.
☐ Lecture : extraire au minimum les points et les couleurs ; conserver les données non comprises dans la table d'extensions pour un aller-retour JEF+ → JEF+ à l'identique.
☐ Écriture : produire un JEF+ que les machines et logiciels Janome acceptent ; tant que la validation sur matériel n'est pas faite, la fonctionnalité est marquée **expérimentale** dans l'interface.
☐ Si le JEF+ permet de stocker plus d'informations de couleur que le JEF, les exploiter pour améliorer la fidélité.

### 4.5 Tests spécifiques JEF / JEF+
◐ Aller-retour JEF → pivot → JEF identique octet pour octet sur le corpus de référence (hors champ date, normalisé dans les tests).
◐ Matrice de conversion : chaque format P0 → JEF, comparée à des fichiers attendus (golden files).
◐ Fixtures de **couleurs proches** :
  - dégradé de 8 bleus avec ΔE00 de 1,5 à 3 entre voisins ;
  - deux blancs cassés différents ;
  - deux noirs (noir et gris anthracite très sombre) ;
  - motif contenant plus de fils distincts que la palette (vérifier les fusions signalées).
  Vérifier : aucune fusion non signalée, ordre de clarté conservé en mode contraste, rapport correct.
☑ Fixture asymétrique pour le sens des axes.
◐ Fuzzing du lecteur JEF et JEF+.
◐ **Recette sur machine réelle** (checklist et fichiers d'essai prêts, essai à réaliser) : checklist manuelle dans `docs/acceptance/janome.md` (chargement sur la machine, affichage du cadre, nombre de couleurs, rendu brodé comparé à l'original). Les résultats sont consignés avec le modèle de machine et la version du firmware.

---

## 5. Interfaces de conversion (P0)

### 5.1 Ligne de commande (`emb-cli`)
◐ `emb convert entree.pes sortie.jef` avec options : stratégie de couleur, cadre, machine cible, nuancier, seuils ΔE, rapport.
☐ Conversion par lot d'un dossier entier, avec rapport consolidé.
☑ `emb info fichier` : format détecté, version, statistiques, couleurs.
☐ `emb validate fichier --machine <modele>` : vérifie la compatibilité avec une machine.
☐ `emb diff a.jef b.jef` : comparaison géométrique et colorimétrique de deux fichiers.
☑ Codes de sortie distincts : succès, succès avec avertissements, échec.

### 5.2 Application graphique et interface MCP
La conversion d'un fichier existant est aussi accessible depuis l'application (ouvrir un PES, l'afficher, l'exporter en JEF : FEATURES §8) et par les outils MCP (FEATURES §6). Les trois interfaces appellent le même service (`emb_formats::convert`, [ADR 0002](adr/0002-interface-mcp.md)) : aucune logique de conversion hors des bibliothèques.

☐ Aperçu avant/après avec la table de correspondance des couleurs éditable (P1).
☐ Affichage du rapport de conversion (P1).
☐ Glisser-déposer de fichiers ou de dossiers, conversion par lot (P2).

---

## 6. Exigences propres à la conversion

### 6.1 Performance
☐ Conversion d'un design de 100 000 points en moins de 200 ms (hors I/O disque), mesurée par benchmark.
☐ Conversion par lot parallélisée (`rayon`).

### 6.2 Qualité et non-régression
☐ Corpus de référence versionné dans `tests/fixtures/` : au moins 5 fichiers réels par format P0, de sources variées et redistribuables.
◐ Matrice de conversion automatisée : chaque format lisible → chaque format écrivable, avec vérification des invariants (géométrie, nombre de blocs, couleurs) et snapshots du rapport de conversion.
☐ Tout fichier qui a posé problème devient une fixture de régression.

### 6.3 Réutilisation (P2)
☐ Bibliothèque de conversion utilisable seule (crate publiable) et, plus tard, en WebAssembly.

Les jalons sont dans FEATURES §11.
