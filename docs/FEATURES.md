# Spécification des fonctionnalités — Logiciel de broderie professionnel

> Document de référence fonctionnelle. À lire avec `rules.md` (règles de code et de tests).
> Chaque fonctionnalité porte une priorité : **P0** (indispensable, premier jalon) → **P3** (plus tard).
> Statut : ☐ à faire · ◐ en cours · ☑ terminé.

---

## 0. Vision et priorité absolue

**La fonctionnalité prioritaire est la conversion fidèle entre tous les formats de broderie des différentes marques de machines.**

Le logiciel doit être capable de lire un fichier de n'importe quelle marque et de l'écrire dans n'importe quel autre format **sans perte perceptible** : même géométrie au dixième de millimètre près, même nombre et même ordre de blocs de couleur, mêmes coupes et sauts, et des couleurs de fil restituées le plus fidèlement possible, **y compris lorsque plusieurs fils ont des couleurs proches**.

Une attention toute particulière est portée à l'export vers **JEF** et **JEF+** (Janome).

### Critères de réussite globaux
- Une conversion A → B → A ne dérive pas : écart de position ≤ 0,1 mm sur chaque point, sans accumulation d'erreur.
- Aucun bloc de couleur n'est fusionné, supprimé ou réordonné.
- Deux fils distincts dans le fichier source restent distincts dans le fichier cible, sauf impossibilité signalée explicitement.
- Chaque conversion produit un **rapport de conversion** listant toute perte ou approximation.

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

## 2. Formats à supporter

Légende : **L** lecture · **É** écriture.

### 2.1 P0 — Formats prioritaires
| Marque | Format | L | É | Remarques |
|---|---|---|---|---|
| Janome / Elna / Kenmore | **.jef** | ◐ | ◐ | Couleurs = index dans la palette Janome. Voir §4. |
| Janome | **.jef+** | ☐ | ☐ | Format étendu peu documenté. Voir §4.6. |
| Brother / Baby Lock / Bernina deco | .pes | ◐ | ☐ | Plusieurs versions (v1 à v6) ; contient une section PEC. Écrire au minimum v1 (compatibilité) et une version récente avec fils RGB. |
| Brother | .pec | ☐ | ☐ | Couleurs = index dans la palette Brother. |
| Tajima | .dst | ◐ | ◐ | Pas d'information de couleur ; coupes encodées par séquences de sauts. Fichier de couleurs annexe optionnel. |
| Melco / Bernina | .exp | ☐ | ☐ | Pas de couleur dans le fichier. |
| Husqvarna Viking / Pfaff | .vp3 | ☐ | ☐ | Fils RGB avec nom, marque et référence catalogue. |
| Singer | .xxx | ☐ | ☐ | Couleurs RGB. |

### 2.2 P1 — Formats courants
| Marque | Format | L | É |
|---|---|---|---|
| Husqvarna Viking | .hus, .vip, .shv | ☐ | ☐ |
| Pfaff | .pcs, .pcd, .pcm, .pcq | ☐ | ☐ |
| Janome | .sew, .jpx | ☐ | ☐ |
| Brother | .phc, .phb | ☐ | — |
| Tajima | .dsb, .dsz, .tbf | ☐ | ☐ |
| Barudan | .u01 | ☐ | ☐ |
| Toyota | .10o, .100 | ☐ | ☐ |
| Happy | .tap | ☐ | ☐ |
| ZSK | .z00 / .zsk | ☐ | ☐ |
| Melco | .cnd | ☐ | — |
| Poem / Singer | .csd | ☐ | ☐ |

### 2.3 P2 — Formats d'échange et utilitaires
☐ Export SVG (tracé des points), PNG (aperçu), PDF (fiche de production avec couleurs et ordre de couture).
☐ Import/export CSV et JSON (format texte lisible, utile pour le débogage et les tests).
☐ Fichiers de couleurs annexes : `.inf`, `.edr`, `.rgb`, `.col`.

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

## 4. Conversion vers JEF et JEF+ (P0 — soin particulier)

### 4.1 Contraintes du format JEF à respecter
☑ Coordonnées en 0,1 mm, deltas signés sur un octet ; tout déplacement plus long est découpé.
◐ Commandes de contrôle (changement de couleur, déplacement, fin) encodées selon la fiche `docs/formats/jef.md`, établie et vérifiée sur des fichiers produits par les logiciels Janome.
☑ Axe Y : vérifier le sens par rapport au modèle pivot avec une fixture asymétrique (ex. la lettre « F ») — un motif inversé doit faire échouer un test.
◐ En-tête cohérent : décalage des données, date, nombre de couleurs, nombre de points, code du cadre et emprises (extents) calculées à partir du motif réel.
☑ Table des couleurs : un index de palette Janome par bloc, dans l'ordre exact des blocs, plus le champ de type de fil associé.

### 4.2 Choix du cadre (hoop)
◐ Sélection automatique du plus petit cadre Janome contenant le motif, centré.
☐ Choix manuel ou par modèle de machine cible (profils de machines Janome avec leurs cadres disponibles).
☑ Erreur explicite si le motif dépasse le plus grand cadre de la machine ciblée (jamais de troncature silencieuse).

### 4.3 Fidélité des couleurs vers JEF
Le JEF ne stocke qu'un index dans la palette Janome : c'est là que les couleurs proches risquent d'être confondues.

☑ Utiliser par défaut l'**affectation optimale injective** (§3.4) sur la palette Janome.
☑ Si la source contient des références de fils Janome, les reprendre telles quelles (correspondance exacte, ΔE ignoré).
☐ Si la source contient des références d'autres marques, passer par les tables de correspondance de nuanciers avant de recourir au RGB.
☑ Ne **jamais** fusionner deux blocs consécutifs, même s'ils obtiennent le même index : chaque changement de couleur source reste un arrêt machine (l'utilisateur peut avoir changé de fil volontairement).
☐ Option « couleurs exactes » : générer à côté du JEF une **fiche de fils** (PDF/HTML/CSV) indiquant pour chaque bloc l'index Janome affiché par la machine **et** le fil réel d'origine (marque, référence, nom, échantillon de couleur), afin que la brodeuse enfile les vraies couleurs.
☐ Aperçu côte à côte dans l'interface : couleurs d'origine / couleurs Janome attribuées, avec mise en évidence des paires de fils proches et de leur ΔE.

### 4.4 JEF+
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
☐ **Recette sur machine réelle** : checklist manuelle dans `docs/acceptance/janome.md` (chargement sur la machine, affichage du cadre, nombre de couleurs, rendu brodé comparé à l'original). Les résultats sont consignés avec le modèle de machine et la version du firmware.

---

## 5. Interface de conversion (P0)

### 5.1 Ligne de commande (`emb-cli`)
◐ `emb convert entree.pes sortie.jef` avec options : stratégie de couleur, cadre, machine cible, nuancier, seuils ΔE, rapport.
☐ Conversion par lot d'un dossier entier, avec rapport consolidé.
☑ `emb info fichier` : format détecté, version, statistiques, couleurs.
☐ `emb validate fichier --machine <modele>` : vérifie la compatibilité avec une machine.
☐ `emb diff a.jef b.jef` : comparaison géométrique et colorimétrique de deux fichiers.
☑ Codes de sortie distincts : succès, succès avec avertissements, échec.

### 5.2 Interface graphique (P1)
☐ Glisser-déposer de fichiers ou de dossiers, choix du format cible, conversion par lot.
☐ Aperçu avant/après avec la table de correspondance des couleurs éditable.
☐ Affichage du rapport de conversion.

### 5.3 Interface MCP (P1)
L'application est **« MCP native »** : un modèle de langage (LLM) doit pouvoir l'utiliser directement, via le Model Context Protocol, avec les mêmes garanties de fidélité que la CLI. Approche : [ADR 0002](adr/0002-interface-mcp.md).

☐ Serveur MCP (crate `emb-mcp`, transport stdio) exposant des outils : `list_formats`, `info`, `convert`, `match_colors`, puis `validate` et `diff`.
☐ Aucune logique propre au serveur : il appelle le même service de conversion que la CLI (détection du format, options, rapport).
☐ Résultats structurés : rapport de conversion JSON (§1.4) avec **codes d'avertissement stables** (ex. `thread_approximated`) ; schémas JSON des entrées et sorties de chaque outil.
☐ Sécurité : accès aux fichiers limité à des dossiers autorisés, taille et durée maximales par appel, aucun écrasement de fichier sans paramètre explicite.
☐ Déterminisme et absence de toute interaction (aucune question posée en cours d'opération).
☐ Tests de bout en bout : appels d'outils sur les fixtures, comparés à des snapshots.

---

## 6. Visualisation (P1)

☐ Aperçu réaliste : épaisseur et brillance du fil, relief, ombrage.
☐ Mode « plan » : points, sauts et coupes en couleurs distinctes.
☐ Simulation de couture animée avec vitesse réglable.
☐ Statistiques : temps de broderie estimé, consommation de fil par couleur, nombre de coupes.
☐ Zoom, mesure de distances, affichage du cadre.

---

## 7. Édition de base (P2)

☐ Déplacement, rotation, mise à l'échelle (avec recalcul de la densité, pas une simple homothétie des points).
☐ Réordonnancement, fusion et suppression de blocs de couleur.
☐ Combinaison de plusieurs designs.
☐ Ajout de texte avec polices de broderie.
☐ Annuler / rétablir illimité.

---

## 8. Numérisation (digitizing) (P3)

☐ Import SVG et images matricielles.
☐ Génération de points : droit, triple, satin, remplissage tatami, avec angle et densité.
☐ Sous-couches (underlay), compensation de tirage, optimisation de l'ordre de couture.
☐ Numérisation automatique assistée à partir d'une image.

---

## 9. Exigences transverses

### 9.1 Performance
☐ Conversion d'un design de 100 000 points en moins de 200 ms (hors I/O disque), mesurée par benchmark.
☐ Conversion par lot parallélisée (`rayon`).

### 9.2 Qualité et non-régression
☐ Corpus de référence versionné dans `tests/fixtures/` : au moins 5 fichiers réels par format P0, de sources variées.
◐ Matrice de conversion automatisée : chaque format lisible → chaque format écrivable, avec vérification des invariants (géométrie, nombre de blocs, couleurs) et snapshots du rapport de conversion.
☐ Tout fichier qui a posé problème devient une fixture de régression.

### 9.3 Plateformes
◐ Windows, macOS, Linux.
☐ Bibliothèque de conversion utilisable seule (crate publiable) et, plus tard, en WebAssembly.

### 9.4 Internationalisation
☐ Interface en français et en anglais au minimum ; noms de fils dans la langue du nuancier.

---

## 10. Jalons proposés

1. **Jalon 1** (◐ en cours — reste le corpus réel et les points « à vérifier », voir `docs/TODO.md`) : modèle pivot, quantification, lecture/écriture DST et JEF, CLI `convert`/`info`, matrice de tests.
2. **Jalon 2** : PES/PEC, EXP, VP3, XXX ; gestion des couleurs complète (CIEDE2000, affectation injective, nuanciers) ; rapport de conversion (JSON à codes stables, service de conversion partagé : prérequis de l'interface MCP).
3. **Jalon 3** : investigation puis support JEF+ ; fiche de fils ; recette sur machines Janome.
4. **Jalon 4** : formats P1, interface MCP (§5.3), interface graphique de conversion, aperçu.
5. **Jalon 5** : édition, puis numérisation.
