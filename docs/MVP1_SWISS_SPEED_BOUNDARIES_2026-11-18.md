# MVP 1: velocità Swiss, stazioni ed estremi

**Stato: aperto; non approvato per il rilascio.** L'utente ha scelto come
riferimento la velocità pubblicata da Swiss Ephemeris, anche quando essa e la
derivata numerica delle posizioni Swiss non coincidono. La versione sperimentale
del motore restituisce una velocità coerente con la derivata delle sue posizioni:
il comportamento attuale non soddisfa ancora la scelta nei casi qui evidenziati.
Stelle e server principale restano fuori da questa fase.

## Cosa è stato confrontato

La prova usa DE441 locale per entrambi i calcoli, Swiss 2.09.03, posizioni
apparenti geocentriche e la variante sperimentale eclittica mista
`experimentalLongTermDirectObliquityTdb`. I tempi del motore sono TDB e quelli
Swiss TT approssimato dalla conversione indicata nello script; le date sono nel
calendario gregoriano prolettico e 13000 a.C. corrisponde all'anno astronomico
−12999. Sole, Luna, Mercurio, Venere e baricentri dei sistemi Marte–Plutone
sono inclusi. La prova misura compatibilità con Swiss usando la stessa soluzione
orbitale, non validazione indipendente della realtà fisica o della Terra antica.

- **114 posizioni/velocità:** 20 ai due estremi richiesti, 80 a date interne
  e 14 geometrie vicine al Sole da un precedente rapporto. Gli estremi sono
  il primo istante del 13000 a.C. e l'ultimo numero `f64` TDB rappresentabile
  prima del 4000 d.C.; non sono una prova continua sull'intero intervallo.
- **Posizioni:** massimo scarto angolare 0,002039″ sui 114 casi, nessuno oltre
  1″; sui 20 estremi il massimo è lo stesso. Il limite di 1″ è stato scelto
  per le posizioni e non vale come tolleranza per velocità o orari.
- **Velocità pubblicate:** differenza massima in longitudine 0,278888″/giorno,
  in latitudine 0,072986″/giorno e in distanza 0,000072131 UA/giorno sui 114
  casi. Nei 14 casi vicini al Sole il massimo in longitudine è
  0,029229″/giorno. Queste grandezze hanno unità diverse e non definiscono da
  sole una soglia di accettazione.
- **48 stazioni:** otto corpi da Mercurio a Plutone, due direzioni e tre epoche
  iniziali (−12500, 0 e 3900). A ±120 secondi dalla radice numerica della
  derivata delle posizioni Swiss, il segno del motore concorda con tale
  derivata in 48 casi; concorda con la velocità pubblicata Swiss in 44 casi.
  I quattro disaccordi riguardano Plutone alle prime due epoche e entrambe le
  direzioni. La maggiore differenza fra i centri delle parentesi numeriche
  motore/velocità pubblicata è circa **754 secondi (12 minuti e 34 secondi)**.
  La bisezione usa una larghezza di 0,00001 giorni (0,864 secondi); non prova
  l'unicità fisica della stazione né l'accuratezza storica dell'orario.
  Il confronto del segno è diretto tra motore e Swiss, anche quando entrambi
  i campioni cadono dallo stesso lato della stazione pubblicata. Una revisione
  dello script ha corretto un precedente controllo che richiedeva erroneamente
  a entrambi i segni Swiss di seguire anche la stazione della derivata; con il
  motore attuale il risultato rimane 44/48.

Le API ordinarie `calcTdb` rifiutano entrambi gli estremi; il percorso esteso
è esplicito e sperimentale. Il controllo esteso rifiuta date ben oltre la sua
guardia numerica e input invalidi, ma accetta gli istanti immediatamente fuori
dal perimetro richiesto: il confine del prodotto non è un limite imposto da
questa API.

## Diagnostica radiale, non correzione rilasciata

Una stima sperimentale indipendente, basata sulla proiezione dell'accelerazione
dell'osservatore e sul ritardo di propagazione, riduce lo scarto radiale rispetto
alla velocità pubblicata in 102 dei 114 campioni: RMS da circa 2,08×10⁻⁵ a
5,06×10⁻⁷ UA/giorno. Nei 14 casi vicino al Sole migliora tutti i campioni.
È una diagnosi approssimata, non dimostra il procedimento interno di Swiss e
non corregge le velocità angolari o le quattro stazioni. Non è stata applicata
al core né proposta come precisione garantita.

Un successivo [esperimento radiale nel core](EXPERIMENTAL_RADIAL_RATE_CANDIDATE_2026-11-18.md)
restituisce separatamente un candidato e lascia invariata la derivata. Il Sole
è escluso esplicitamente; le velocità angolari e le stazioni restano aperte.

## Riproduzione e gate

```text
node bindings/node/scripts/validate-mvp1-swiss-rates.cjs DE441 DE441_SHA256 SWISSEPH_PACKAGE NEAR_SUN_REPORT NEAR_SUN_SHA256 NEW_REPORT
```

Il file DE441 e Swiss sono forniti dal chiamante; nessuno è una dipendenza
runtime della libreria. Il rapporto locale non tracciato contiene hash degli
input e dello script. Lo script scrive il rapporto anche quando il controllo
fallisce e restituisce codice 2 in caso di posizioni oltre 1″, stazioni non
risolte o disaccordi di segno con la velocità pubblicata. **Il risultato
attuale è 2** per i quattro disaccordi. Anche l'eventuale assenza di questi
fallimenti non chiuderebbe automaticamente il gate: restano da concordare
tolleranze numeriche distinte per velocità e tempi delle stazioni.

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Rapporto geometrie vicino al Sole SHA-256: `1e48b86bc7f4b242c570a503a41b8372e2f9d8d580a4b02d3925def6f41b2b50`.
- Rapporto della prova con confronto di segno corretto SHA-256: `5a56663cf82bca72bb3c6a2e1e2cdc13f703926d600f1d67b5123cc2b28dfd4b`.

Prossima azione: sviluppare nel core una modalità esplicita di velocità
compatibile con il riferimento scelto, preservando il contratto della derivata
attuale; verificare separatamente componenti angolari e radiale, le quattro
stazioni fallite e casi non usati per formulare la correzione. Stabilire le
tolleranze mancanti prima di dichiarare chiuso questo gruppo della MVP.
