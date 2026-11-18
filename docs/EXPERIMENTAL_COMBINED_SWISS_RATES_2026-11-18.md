# Candidati combinati di velocità — 18 novembre 2026

**Sperimentale, non ancora accettato per la MVP.** Il percorso combina i
[candidati in longitudine](EXPERIMENTAL_LONGITUDE_RATE_CANDIDATE_2026-11-18.md)
e [in distanza](EXPERIMENTAL_RADIAL_RATE_CANDIDATE_2026-11-18.md), conservando
anche i risultati dei due metodi separati. Non cambia le API ordinarie e non
attiva il server principale. I due termini sono ipotesi empiriche sviluppate
indipendentemente; non descrivono l'implementazione interna di Swiss.

`experimental_combined_rate_candidate` nel core e
`experimentalCombinedRateCandidateTdb` nel binding Node richiedono un JD TDB
e un corpo. Nel binding:

- `base.xx` contiene le sei grandezze originali, comprese le velocità che sono
  derivate delle posizioni del motore;
- `candidateXx` mantiene posizioni e velocità di latitudine di `base`, ma
  usa i candidati sperimentali in longitudine e distanza: **queste due
  velocità non vanno presentate come derivate delle posizioni**;
- `radialCorrectionApplied` è `false` per il Sole, escluso esplicitamente.

Il core verifica che i due calcoli separati producano la stessa base e fallisce
in caso contrario. L'attuale implementazione ricalcola parte della pipeline:
è progettata per la verifica, non è ancora ottimizzata per un uso intensivo.
La latitudine non ha una correzione di compatibilità in questo percorso.

Un successivo [candidato sperimentale in latitudine](EXPERIMENTAL_LATITUDE_MOON_RATES_2026-11-18.md)
è esposto tramite un metodo a tre velocità distinto; questo metodo a due
velocità mantiene il proprio contratto.

## Controllo della combinazione

È stato ripetuto il confronto con Swiss 2.09.03/JPL e DE441 locale comune,
in coordinate apparenti geocentriche e variante mista direct-obliquity; il
motore usa TDB e Swiss TT approssimato. La prova riusa i campioni esistenti,
non aggiunge osservazioni indipendenti e non prova un limite continuo:

- Nelle finestre di **96 stazioni** selezionate in precedenza, la concordanza
  dei segni a ±120 secondi è **96/96** per il combinato, contro **90/96**
  per la velocità originale. Le quattro stazioni di Plutone del primo gruppo
  avevano orientato l'ipotesi angolare.
- Nei **320 casi radiali** di conferma precedenti, il candidato combinato
  migliora lo scarto in 276 casi; la RMS passa da 2,195×10⁻⁵ a 4,515×10⁻⁸
  UA/giorno. Il Sole è invariato; alcuni casi della Luna peggiorano.
- Per ogni campione esaminato, il confronto verifica esattamente che la base
  e i due candidati del percorso combinato coincidano con i metodi separati.
  Non ricalcola le radici delle stazioni: per quelle valgono i limiti e le
  risoluzioni descritti nel rapporto angolare.

Rimangono aperti la compatibilità in latitudine, le differenze per la Luna e
altri corpi, le tolleranze per velocità e tempi di stazione, il contratto del
tempo storico e l'integrazione Desktop. Non è una certificazione di MVP né
una garanzia dell'orario civile per epoche antiche.

## Riproduzione

Il comando è offline. Richiede DE441, Swiss e tre rapporti locali identificati
da SHA-256, oltre a un percorso di output nuovo:

```text
node bindings/node/scripts/verify-combined-swiss-rates.cjs DE441 DE441_SHA SWISSEPH STATIONS_1 SHA STATIONS_2 SHA RADIAL_GRID SHA NEW_REPORT
```

- DE441: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`
- Stazioni iniziali: `8580dd62db0797063e72a7786a517a4772999da5eb6decc718c20d232cbd87b4`
- Stazioni aggiuntive: `9b3cd2af109044e04c5624bd0590a3c060b13befbe1d358836f1007fae3af5d7`
- Griglia radiale di conferma: `d40d33dbdc46e9a75f16cb96d36bbb7958e3760a1308f5ee200f570a926c0efc`
- Rapporto combinato: `7b4cafc94d11ec3047856ed3716fd84127fc3c749a6fd0eafcd6f5c0f41df99e`

I rapporti e il kernel restano locali e non tracciati. Swiss viene utilizzato
solo nello script di verifica: non è una dipendenza del core o del binding.
