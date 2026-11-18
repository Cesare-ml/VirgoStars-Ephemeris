# MVP 1 pianeti: punto di arrivo e lavoro rimanente

**Stato: candidato sperimentale, non ancora MVP rilasciata.** Questo documento
fissa un percorso finito sulla base delle decisioni dell'utente del 18 novembre
2026. Le stelle e la migrazione del server principale rimangono in pausa.
Non trasforma controlli campionati in una garanzia di accuratezza fisica.

## Decisioni già prese

- Intervallo richiesto: **13000 a.C. – 3999 d.C.**; con la numerazione
  astronomica usata dai test, 13000 a.C. è l'anno −12999. Le date delle griglie
  sono nel calendario gregoriano prolettico.
- Per le coordinate eclittiche, **Swiss Ephemeris è il riferimento di
  compatibilità**. Non è una dichiarazione che Swiss misuri l'effettiva
  orientazione della Terra antica.
- Per le posizioni campionate rispetto a Swiss, l'utente ha scelto uno scarto
  massimo di **1 secondo d'arco**. Il confronto qui misura la separazione
  angolare sulla sfera, non soltanto la differenza di longitudine; non è una
  garanzia continua fra le date campionate o una misura dell'errore storico.
- Questo limite riguarda le **posizioni**: non è una tolleranza di velocità,
  un limite sui tempi delle stazioni, né un'incertezza di ΔT.
- Per le velocità e i cambi di moto, l'utente ha scelto la compatibilità con
  **la velocità pubblicata da Swiss**, anche quando differisce dalla derivata
  numerica delle posizioni Swiss. La compatibilità non è ancora raggiunta:
  si veda [velocità, stazioni ed estremi](MVP1_SWISS_SPEED_BOUNDARIES_2026-11-18.md).
- Per la **sola velocità in longitudine della Luna**, l'utente ha accettato
  per la MVP uno scarto massimo **campionato** di 0,3″/giorno rispetto a Swiss.
  Il [controllo di 194 casi](MVP1_LUNAR_LONGITUDE_SPEED_2026-11-18.md) lo
  soddisfa; non definisce una tolleranza per gli altri corpi o altre componenti.
- Per i **96 cambi di moto campionati**, l'utente ha accettato un massimo di
  **15 secondi TDB** rispetto agli zeri numerici delle velocità pubblicate
  Swiss. Il [controllo riproducibile](MVP1_STATIONS_AND_HISTORICAL_RATES_2026-11-18.md)
  soddisfa quel limite e verifica i segni; non è un vincolo continuo o una
  garanzia sugli orari civili storici.
- Per gli altri campioni di **velocità angolare**, l'utente ha scelto un
  massimo di **0,1″/giorno**; per i campioni di **velocità radiale**, un
  massimo di **0,0000005 UA/giorno**. I controlli e i rispettivi perimetri
  sono descritti nei [criteri campionati](MVP1_SAMPLED_SPEED_ACCEPTANCE_2026-11-18.md).

Il perimetro base qui verificato è la direzione apparente geocentrica di
Sole, Luna, Mercurio, Venere e baricentri dei sistemi Marte–Plutone, con
DE441 fornito dal chiamante. Nodi, apogei, Chirone, case e sorgere/tramontare
hanno contratti e coperture proprie: non sono promossi automaticamente per
l'intero intervallo da questo risultato. Se devono far parte della stessa
consegna, vanno elencati esplicitamente con i loro limiti prima del rilascio.

## Posizioni: controllo campionato superato

Il 18 novembre è stata riesaminata la coppia di rapporti già registrati,
verificando hash, date, corpi, flag Swiss e coerenza della scomposizione.
Per la variante sperimentale **direct-obliquity**, la separazione massima
dal risultato Swiss su **5.480 casi è 0,002232″**; i casi oltre 1″ sono
**zero**. Dei 5.480, 1.000 sono anteriori al tratto disponibile su Horizons;
in questo sottoinsieme il massimo è 0,002068″. I confronti usano la stessa
soluzione DE441. Non abbiamo dimostrato un limite continuo né la precisione
fisica della Terra antica. La variante è una convenzione mista: la scelta
dell'utente riguarda la compatibilità, non una superiorità fisica dimostrata.

Questo risultato **non cambia il percorso predefinito**. Le API `calcTdb` e
`calcUt` normali mantengono i loro controlli moderni; la copertura estesa
usa metodi esplicitamente sperimentali. Un consumatore deve selezionare e
documentare il percorso compatibile con Swiss, senza fallback nascosto.
La griglia contiene quattro stagioni per anno selezionato, non ogni giorno:
per la chiusura vanno verificati esplicitamente anche gli estremi dell'intero
intervallo e gli errori appena fuori copertura.

Il controllo finito riutilizza le evidenze: non contatta Swiss o Horizons e
non modifica la libreria. Richiede una soglia esplicita, verifica le due
griglie e restituisce un rapporto condizionale; se una posizione eccede la
soglia scrive il rapporto e restituisce codice 2. Non certifica da solo la MVP.

```text
node bindings/node/scripts/assess-mvp1-swiss-positions.cjs BASELINE BASELINE_SHA INTERLEAVED INTERLEAVED_SHA 1 NEW_REPORT
```

- Rapporto sorgente baseline: `8315c9a1e21737db697a01f6413600433c2e58c331f907af6b6abc186852e5a1`.
- Rapporto sorgente interleaved: `8eb838d223a66023cbc7c11b1b7050b7fd154563e3dc23a156415e14c4c2ee62`.
- Rapporto del controllo con soglia 1″: `e84ac4c07493f4b69e6388fa72ccf63b2d1e08adee14e3adda83cbb389d34290`.

I file di confronto e DE441 restano locali e non tracciati. Per verificare
che il controllo fallisca davvero, una prova tecnica a 0,001″ ha rilevato
190 superamenti e restituito codice 2: non è il limite scelto per la MVP.

## Tre gruppi di verifiche rimasti

1. **Velocità, cambi di moto ed estremi — criteri campionati superati; integrazione aperta.** Usare Swiss come confronto con
   convenzioni esplicite; riutilizzare i risultati già raccolti e chiudere i
   casi mancanti sulla variante direct-obliquity, sui confini e vicino al
   Sole. Definire separatamente unità e tolleranze di velocità e di tempo
   delle stazioni: il limite di posizione di 1″ non le definisce. Le prove
   sulle posizioni e le precedenti stazioni non dimostrano ancora questo gate.
   Il controllo successivo copre i due estremi e individua quattro disaccordi
   di segno su 48 stazioni rispetto alla velocità pubblicata originale. Un
   candidato sperimentale li ha corretti nei campioni. I controlli alle soglie
   scelte dall'utente sono superati per le 96 stazioni, la velocità lunare in
   longitudine, le altre componenti angolari e le componenti radiali nei
   rispettivi campioni. Le griglie differiscono e non garantiscono ogni data;
   la selezione del percorso nel consumer e il controllo end-to-end restano
   aperti.
2. **Contratto del tempo storico.** Per le date fuori dalla copertura moderna,
   usare un input UT1/TT/TDB chiaramente identificato e una politica esplicita
   per ΔT e la sua fonte/incertezza. Riutilizzare i test delle tre ipotesi e
   verificare estremi e dati mancanti; non descrivere UTC o un orario civile
   antico come UT1 esatto. Un nuovo percorso sperimentale collega UT1 e ΔT
   espliciti ai tre candidati di velocità, ma non sceglie un modello storico
   o una fonte per il chiamante. Non serve inventare una precisione storica
   che le osservazioni non consentono.
3. **Consumo riproducibile in Desktop.** Aggiornare e verificare in modo
   isolato la revisione del core, i dati DE441, gli errori e il contratto delle
   date, poi eseguire i controlli end-to-end sul percorso effettivo. Il
   `main` Desktop osservato al commit `47bda2623c85d49f30f8d111d9f7245de4525d8c`
   punta ancora al core `6835e00e4b4ba5fc52d882a793e2dfa2b2a436aa` e limita
   le richieste a **1900–2033**, con ulteriori limiti dei dati ΔT: quindi il
   nuovo intervallo non è già disponibile attraverso quel provider. La
   compilazione locale, il pacchetto e la distribuzione restano stati distinti.

Sono tre **gruppi** di chiusura, non tre singoli test né una promessa che un
test fallito non richieda una correzione. Non si aggiungono ulteriori griglie
esplorative solo per cercare più decimali: si ripete o amplia una prova quando
serve a chiudere uno di questi punti o a risolvere un fallimento concreto.

## Test eseguiti in questo checkpoint

- Con DE441, Swiss e i metodi delle case al checkout pulito
  `astrological-functions` `5b4df7dce4e00fa556fc22500e08295c723ec9f5`, la
  suite Node ha eseguito **172 test, tutti passati, con 15 saltati**. La prova
  delle case da sola ha eseguito 11 test senza salti. Il checkout storico è
  una dipendenza locale delle prove, non dimostra che il `main` corrente di
  `astrological-functions` abbia lo stesso contratto.
- I 15 salti includono stelle, backend isolato, dati Chirone e tabelle USNO
  mancanti nella configurazione usata. Non sono test passati; i primi due
  filoni restano fuori da questa fase quando dipendono dalla migrazione del
  server o dalle stelle.
- La CI della PR #627 è passata; la ripetizione locale della sua griglia
  baseline ha prodotto il medesimo SHA-256 registrato.
- La suite JavaScript alla radice ha eseguito 105 test, tutti passati, con
  9 saltati per prerequisiti opzionali.

La futura sostituzione del server principale richiede ancora il catalogo
stellare completo richiesto dall'utente e un percorso separato di integrazione:
non è compresa nei tre gruppi del traguardo pianeti/Desktop sopra.
