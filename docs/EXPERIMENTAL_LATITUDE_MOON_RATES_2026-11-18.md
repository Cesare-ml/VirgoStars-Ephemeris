# Velocità in latitudine e scarti lunari — 18 novembre 2026

**Stato: sperimentale, non accettato per la MVP.** Questo lavoro segue i
[candidati combinati](EXPERIMENTAL_COMBINED_SWISS_RATES_2026-11-18.md).
La compatibilità richiesta è con la velocità pubblicata da Swiss Ephemeris;
non è una certificazione della precisione fisica o del tempo civile antico.

## Candidato in latitudine

La relazione geometrica usata come ipotesi per la longitudine suggerisce il
termine `sin(longitudine) × d(Δε)/dt` per la latitudine. Il core valuta la
derivata numerica della propria serie di nutazione estrapolata. Il termine è
un'ipotesi empirica: non dimostra la procedura interna di Swiss e la velocità
candidata non è la derivata della posizione restituita.

Il metodo esplicito `experimentalLatitudeRateCandidateTdb` restituisce base,
correzione e candidato separati. `experimentalThreeRateCandidateTdb` restituisce
anche i candidati già sviluppati per longitudine e distanza; `base.xx` mantiene
le sei grandezze originali e `candidateXx` contiene le tre velocità
sperimentali. Il precedente metodo a due velocità, le API normali e il server
principale restano invariati. Il Sole è ancora escluso dalla correzione
radiale e il flag lo segnala. L'implementazione sperimentale ripete diversi
calcoli: non è stata ottimizzata come percorso di produzione.

La prima griglia, usata per individuare il termine, ha mostrato un miglioramento
in 308 casi su 320. Una seconda griglia, con **date diverse** e la formula
implementata nel core, copre otto anni astronomici da −12500 a 3900, quattro
mesi e dieci corpi: Sole, Luna, Mercurio, Venere e baricentri Marte–Plutone.
Il confronto usa coordinate apparenti geocentriche nella variante eclittica
mista direct-obliquity, TDB nel motore e TT approssimato in Swiss 2.09.03;
entrambi usano la stessa DE441 locale.

| Quantità sulla seconda griglia | Prima | Candidato |
| --- | ---: | ---: |
| RMS dello scarto in latitudine | 0,026963″/giorno | 0,003357″/giorno |
| Massimo scarto assoluto campionato | 0,075002″/giorno | 0,024042″/giorno |

Il candidato riduce lo scarto assoluto in **305/320** casi. Per la Luna lo
riduce in **24/32**; i restanti 8 peggiorano. La RMS lunare in latitudine
passa da 0,028592 a 0,010203″/giorno. Gli altri corpi migliorano in 30–32
casi su 32. Sono osservazioni campionate, non limiti continui o tolleranze
accettate: il gate velocità resta aperto.

## Longitudine lunare: causa ancora da verificare

Per 96 date lunari distinte, lo script confronta la **velocità pubblicata da
Swiss con la derivata numerica delle posizioni Swiss dello stesso modo**. I
flag JPL e delle correzioni richieste sono controllati a ogni chiamata. Con
passo di differenza 0,02 giorni TDB, la RMS dello scarto in longitudine è:

| Modo Swiss | RMS ″/giorno |
| --- | ---: |
| Apparente standard | 0,119575 |
| Aberrazione disabilitata | 0,119574 |
| Deflessione disabilitata | 0,119575 |
| Aberrazione e deflessione disabilitate | 0,119574 |
| Posizione vera | 0,001545 |

Passi 0,01 e 0,04 giorni danno RMS rispettivamente circa 0,119479 e
0,119318″/giorno nel modo apparente. Il comportamento è coerente con uno
scarto associato al trattamento della luce, ma **posizione vera cambia più
correzioni insieme**: il confronto non dimostra la causa interna e non
giustifica ancora una correzione lunare. La differenza numerica e l'incertezza
del modello restano distinte dall'incertezza del tempo storico.

## Riproduzione

Lo script offline richiede DE441 e Swiss forniti dal chiamante, più un percorso
di output nuovo; non introduce Swiss come dipendenza runtime:

```text
node bindings/node/scripts/compare-latitude-moon-rates.cjs DE441 DE441_SHA SWISSEPH NEW_REPORT
```

DE441 SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`;
rapporto locale SHA-256 `43941ff2eb4b838ef37184472ff83d7050e4bf70a6e6a9003f47fda62ec7c69e`.
Due esecuzioni hanno prodotto lo stesso hash. Il rapporto include hash del
binding, dello script e dell'installazione Swiss; kernel e risultati restano
locali. Rimangono da risolvere la longitudine lunare e gli altri residui,
concordare limiti di velocità e stazione e completare tempo storico e Desktop.
