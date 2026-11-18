# Candidato sperimentale per la velocità radiale — 18 novembre 2026

**Sperimentale, non accettato per la MVP.** Il core ora espone separatamente
un candidato per la sola velocità radiale, motivato dall'associazione osservata
nel [confronto Swiss](MVP1_SWISS_SPEED_BOUNDARIES_2026-11-18.md). Le velocità
angolari e i cambi di moto restano invariati e non conformi al riferimento
scelto nei quattro casi già osservati. Non è una correzione generale Swiss.

Il termine indipendentemente formulato è `(r · a) / (c × AU)`: posizione
apparente geocentrica ICRF `r`, accelerazione baricentrica della Terra `a`,
velocità della luce `c`, con unità km e giorni TDB. L'accelerazione proviene
dalla derivata analitica dell'interpolante DE441. Non si assume che questa sia
la formula interna di Swiss, né che il candidato sia la derivata della
posizione restituita. Per evitare di cambiare il significato di un risultato
esistente, il metodo restituisce le sei grandezze originali in `base`, il
termine separato e la somma candidata. `correctionApplied` indica se il termine
è applicato. Il percorso normale e le API preesistenti restano invariati.

La prima verifica su 320 combinazioni ha mostrato che il termine peggiorava
tutti i 32 casi del Sole, già molto vicini a Swiss. **Il Sole è escluso
esplicitamente**, con termine zero e `correctionApplied: false`; questa scelta
è stata presa dopo avere visto quei risultati. La Luna è ancora più sensibile:
non tutti i suoi casi migliorano. Non si suggerisce una correzione universale.

## Verifica su date differenti

Dopo la scelta sul Sole è stata usata una seconda griglia di **320 casi**,
date diverse dalla prima: otto anni da −11000 a 3800, quattro mesi, dieci
corpi (per Marte–Plutone, baricentri di sistema). Il confronto è geocentrico
apparente nella variante mista direct-obliquity: TDB per il motore, TT
approssimato per Swiss 2.09.03; entrambi usano lo stesso DE441. È una verifica
di compatibilità campionata, non indipendenza orbitale o accuratezza storica.

- Lo scarto radiale RMS rispetto alla velocità pubblicata Swiss scende da
  **2,195×10⁻⁵ a 4,515×10⁻⁸ UA/giorno** sull'intera griglia. Il massimo
  scende da **8,069×10⁻⁵ a 2,318×10⁻⁷ UA/giorno**.
- Tutti i 256 casi da Mercurio a Plutone migliorano. La Luna migliora in 20
  casi su 32 e peggiora negli altri 12; i 32 casi del Sole restano invariati.
  Il miglioramento aggregato non sostituisce criteri per ciascun corpo.
- La correzione non cambia la longitudine, la sua velocità o le stazioni:
  non può risolvere i quattro disaccordi di segno già individuati.

La griglia di conferma non contiene gli stessi istanti della prima, ma non è
una dimostrazione su tutte le date, né un limite massimo continuo. Mancano
tolleranze concordate di velocità e di orario; non dichiarare il gate chiuso.

## Uso e riproduzione

Il metodo Rust `experimental_radial_rate_candidate` e il corrispondente
`experimentalRadialRateCandidateTdb` nel binding Node richiedono esplicita
selezione. Il secondo restituisce `base`, `correctionAuPerTdbDay`,
`candidateAuPerTdbDay`, `correctionApplied` e avvertenze. L'input è JD TDB;
nessuna conversione di tempo storico o UT1 è implicita.

```text
node bindings/node/scripts/compare-radial-candidate-holdout.cjs DE441 DE441_SHA256 SWISSEPH_PACKAGE NEW_REPORT confirmation
```

Il rapporto registra hash dello script, del binding, di Swiss e del kernel;
non vengono tracciati file planetari o codice Swiss. DE441 SHA-256:
`476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
Rapporto locale di conferma SHA-256:
`d40d33dbdc46e9a75f16cb96d36bbb7958e3760a1308f5ee200f570a926c0efc`.

Resta prioritario risolvere la componente angolare rispetto alla velocità
pubblicata e verificare le stazioni; la stima radiale da sola non abilita
Desktop esteso o la migrazione del server principale.
