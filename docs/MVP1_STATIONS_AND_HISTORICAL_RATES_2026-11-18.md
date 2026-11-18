# MVP 1: stazioni campionate e velocità con UT1 esplicito

**Risultato parziale:** il criterio scelto dall'utente per 96 cambi di moto è
soddisfatto nei campioni; è disponibile un percorso sperimentale UT1 per i tre
candidati di velocità. Non sono ancora chiusi gli altri criteri di velocità,
il contratto completo delle date storiche in Desktop o il rilascio MVP.

## 96 cambi di moto: limite campionato di 15 secondi

L'utente ha accettato un massimo di **15 secondi TDB** fra gli zeri numerici
del candidato e della velocità pubblicata da Swiss. Sui **96 casi** già
raccolti il massimo è **11,536 secondi**; i segni concordano a ±120 secondi
nei 96 casi e non ci sono superamenti del limite. Si tratta di 8 corpi da
Mercurio a Plutone, due direzioni e sei anni di riferimento, non di tutte le
stazioni dell'intervallo. I quattro casi iniziali di Plutone hanno contribuito
alla formulazione del candidato; le 48 stazioni aggiuntive usano semi diversi.

La bisezione precedente aveva larghezza 0,864 secondi, che non equivale a
precisione fisica; Swiss mostra oscillazioni numeriche in alcune finestre. Le
prove usano posizioni apparenti geocentriche nella variante mista eclittica,
DE441 comune, TDB per il motore e TT approssimato per Swiss 2.09.03. Il limite
non è una garanzia continua e non determina l'orario UT1 o civile antico.

Il controllo offline riusa due rapporti locali, ne verifica hash, matrice
completa, campioni, segni e soglia, e restituisce codice 2 se un caso fallisce:

```text
node bindings/node/scripts/assess-mvp1-swiss-stations.cjs FIRST FIRST_SHA SECOND SECOND_SHA DE441_SHA 15 NEW_REPORT
```

Primo rapporto `8580dd62db0797063e72a7786a517a4772999da5eb6decc718c20d232cbd87b4`;
secondo `9b3cd2af109044e04c5624bd0590a3c060b13befbe1d358836f1007fae3af5d7`;
DE441 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`;
risultato a 15 s `c4dbe73991fde06f02a653645d5460744773d01917f37d2702a57f0fb82f68ef`.
Un controllo tecnico a 10 secondi fallisce in tre casi e restituisce codice 2:
10 secondi **non** è il limite scelto dall'utente. I file e il kernel restano
locali e gli hash identificano byte, non autenticità delle fonti.

## Contratto UT1 e candidati di velocità

Il percorso precedente `experimentalLongTermDirectObliquityUt1` restituisce
le derivate originali. Il nuovo `experimentalThreeRateCandidateUt1` permette
invece di richiedere i tre candidati partendo da un JD **UT1** e passando
esplicitamente ΔT = TT−UT1, incertezza e una fonte descrittiva del chiamante.
Il risultato contiene `jdUt1`, `jdTt`, `jdTdb`, ΔT, incertezza e fonte,
`baseXx` con le derivate originali e `candidateXx` con le velocità
sperimentali. Il flag radiale indica se la correzione è applicata; per il Sole
è esclusa. Le velocità sono per giorno TDB, non per giorno civile locale.

Non viene scelto un modello ΔT, né UTC viene trattato implicitamente come UT1.
L'incertezza è un'asserzione del chiamante: non è trasformata in intervallo
sulle posizioni o garanzia statistica. La conversione TT/TDB usa un termine
periodico approssimato; la precisione in epoche remote non è stabilita. Input
non finiti, fonte vuota, incertezza negativa, corpo non supportato o errore di
copertura producono errori espliciti. Il metodo non modifica `calcUt`, il
percorso TDB o il server principale.

I test locali con DE441 verificano gli estremi richiesti, epoche intermedie,
quattro corpi, valori ΔT sintetici, errori e uguaglianza esatta tra il
risultato UT1 convertito e il percorso sperimentale TDB. Sono test di contratto
e integrazione; non dimostrano che il ΔT fornito sia storicamente corretto.
La futura integrazione Desktop deve selezionare la fonte e il trattamento del
tempo civile, presentarne i limiti ed eseguire i controlli end-to-end.
