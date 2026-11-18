# Candidato sperimentale per la velocità di longitudine — 18 novembre 2026

**Stato: sperimentale; gate MVP aperto.** Il riferimento scelto è la velocità
pubblicata da Swiss Ephemeris, distinta dalla derivata della sua posizione.
La [verifica precedente](MVP1_SWISS_SPEED_BOUNDARIES_2026-11-18.md) aveva
individuato quattro disaccordi di segno su 48 stazioni campionate.

È stata osservata nei quattro casi di Plutone una relazione numerica tra lo
scarto e la derivata della nutazione in obliquità. Il candidato usa
`−cos(longitudine) × tan(latitudine) × d(Δε)/dt`, in coordinate eclittiche
della variante mista direct-obliquity. La derivata è calcolata per differenza
centrale dalla serie di nutazione già implementata nel core. Il termine è
un'**ipotesi empirica**, non un'affermazione su come Swiss sia implementato,
un nuovo modello fisico dimostrato o la derivata delle posizioni restituite.

Il metodo Rust `experimental_longitude_rate_candidate` e il binding Node
`experimentalLongitudeRateCandidateTdb` restituiscono la velocità candidata e
la correzione separatamente dalle sei grandezze originali (`base`). L'input è
JD TDB; le velocità sono per giorno TDB, senza conversione UT1 implicita.
Il percorso normale, posizioni, velocità in latitudine e distanza non vengono
modificati. La correzione è rifiutata ai poli e per input non finiti.

## Stazioni campionate

Il confronto usa Swiss 2.09.03 e la stessa DE441 fornita dal chiamante per
entrambi i calcoli. Il motore riceve TDB e Swiss TT approssimato; il centro è
geocentrico, il moto è apparente, Marte–Plutone sono baricentri di sistema.
Non è una validazione indipendente delle orbite o del tempo civile storico.

Per ciascuna stazione si confrontano i segni delle velocità a ±120 secondi
dalla radice della derivata delle posizioni Swiss. Sono poi confrontati i
centri delle parentesi ottenute separatamente per le velocità pubblicate. La
larghezza finale di 0,864 secondi è risoluzione numerica, non una garanzia di
precisione fisica: vicino a zero il risultato Swiss può oscillare.

| Campione | Segni concordi prima | Segni concordi con candidato | Scarto massimo radici prima | Scarto massimo con candidato |
| --- | ---: | ---: | ---: | ---: |
| 48 stazioni iniziali | 44/48 | 48/48 | 753,9 s | 11,6 s |
| 48 stazioni aggiuntive | 46/48 | 48/48 | 319,6 s | 9,9 s |

Le quattro stazioni di Plutone iniziali hanno orientato l'ipotesi, quindi non
sono una prova indipendente di selezione. Le stazioni aggiuntive usano semi
negli anni astronomici −11000, 1500 e 3500, diversi dai primi; comprendono
Mercurio–Plutone e i due versi di cambio. Il candidato riduce lo scarto fra
radici in 31 casi su 48 in ciascun campione: gli altri non vanno descritti come
automaticamente migliorati. Nessuno dei massimi campionati è un limite per
tutti i cambi di moto o per l'incertezza storica.

Su un'ulteriore griglia di 320 velocità, 205 scarti assoluti migliorano.
La RMS globale passa da 0,037259 a 0,037139″/giorno: domina la Luna, dove
rimane una discrepanza rilevante. Per Mercurio la RMS aumenta leggermente;
per Plutone scende da circa 0,007622 a 0,000630″/giorno. Occorre definire
tolleranze distinte per corpi/velocità e per tempi delle stazioni; non è
corretto dichiarare la compatibilità completa sulla base di questi numeri.

## Riproduzione

Tutti i comandi sono offline e richiedono file locali forniti dal chiamante e
nuovi percorsi di output. La selezione aggiuntiva viene effettuata prima del
confronto con il candidato; kernel e Swiss non sono dipendenze runtime.

```text
node bindings/node/scripts/generate-longitude-station-holdout.cjs DE441 DE441_SHA SWISSEPH NEW_SELECTION
node bindings/node/scripts/compare-longitude-candidate-stations.cjs DE441 DE441_SHA SWISSEPH SELECTION SELECTION_SHA NEW_REPORT
node bindings/node/scripts/compare-longitude-candidate-grid.cjs DE441 DE441_SHA SWISSEPH NEW_GRID_REPORT
```

SHA-256 DE441 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`;
selezione aggiuntiva `21fea3789df4207bf92acc8c691e10bacbe5417f966164ce909b9a08d06ebc22`;
rapporto stazioni iniziali `8580dd62db0797063e72a7786a517a4772999da5eb6decc718c20d232cbd87b4`;
rapporto stazioni aggiuntive `9b3cd2af109044e04c5624bd0590a3c060b13befbe1d358836f1007fae3af5d7`;
griglia velocità `f272bf7cc613d76e9f196f469c1018a04ed5b83477f4eb9820d79d1f2dbeefa2`.
I rapporti includono hash del binding, degli script e dell'installazione
Swiss: identificano byte locali e non certificano l'autenticità della fonte.
