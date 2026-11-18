# MVP 1: velocità lunare in longitudine — 18 novembre 2026

**Controllo campionato superato per la sola velocità lunare in longitudine.**
L'utente ha scelto come riferimento la velocità pubblicata da Swiss Ephemeris
e ha accettato per la MVP uno scarto massimo campionato di **0,3″/giorno** per
questa sola grandezza. Non è una soglia per latitudine, distanza, altri corpi,
orari di stazione, ΔT o accuratezza fisica storica. Il resto della MVP rimane
aperto; stelle e server principale sono in pausa.

## Risultato e perimetro

Il metodo esplicitamente sperimentale già presente su `main`,
`experimentalCombinedRateCandidateTdb`, è confrontato con Swiss 2.09.03/JPL:
longitudine apparente geocentrica nella variante eclittica mista
direct-obliquity, input TDB, Swiss TT approssimato, DE441 locale comune. Il
candidato non è una derivata della posizione; l'API restituisce separatamente
anche la derivata originale. Non si tratta di due soluzioni orbitali
indipendenti né di un'approvazione all'attivazione sul server principale.

La griglia contiene **194 istanti**: due date al mese per otto anni astronomici
tra −12999 e 3999 più i due estremi richiesti. Il primo è l'inizio del
13000 a.C. nel calendario gregoriano prolettico; l'ultimo è l'ultimo valore
`f64` TDB prima del 4000 d.C. Per il candidato, lo scarto massimo osservato
è **0,270162″/giorno**, con **zero casi sopra 0,3″/giorno**; la RMS è
0,120643″/giorno. Questo risultato non stabilisce un limite sulle date non
campionate. I valori alle estremità non dimostrano la precisione di un orario
civile storico o di una conversione UT1/ΔT.

Lo scarto rispetto a Swiss non è stato eliminato: la soglia scelta lo rende
esplicitamente accettabile **per questi campioni e questa sola componente**.
Il confronto non promuove automaticamente il candidato a comportamento
predefinito. Rimangono da completare le altre componenti, i criteri per gli
altri corpi e per le stazioni, il contratto del tempo storico e la verifica
Desktop. Il [candidato in latitudine](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/638)
è ancora in una PR separata.

## Perché non applicare un'altra correzione lunare

Sulle stesse date, la differenza RMS fra velocità pubblicata Swiss e derivata
centrale a quattro punti delle sue posizioni è circa **0,120666″/giorno**.
Una differenza unilaterale molto più breve produce uno scarto RMS di circa
**1,1302″/giorno**, con massimo campionato **15,6163″/giorno**: il risultato
dipende sensibilmente dalla scelta numerica e non identifica da solo la causa.
Una prova locale senza affinamento del tempo di emissione non ha fornito un
miglioramento affidabile ed è stata scartata. Non si è dedotto il funzionamento
interno di Swiss né aggiunta una correzione non verificata.

## Riproduzione e controllo del limite

Entrambi i comandi sono offline e richiedono output nuovi; DE441 e Swiss sono
forniti dal chiamante e restano fuori dal repository. Il primo ricrea il
rapporto, il secondo verifica schema, hash, tutte le date attese e soglia:

```text
node bindings/node/scripts/audit-lunar-speed-resolution.cjs DE441 DE441_SHA SWISSEPH NEW_REPORT
node bindings/node/scripts/assess-lunar-longitude-speed.cjs REPORT REPORT_SHA DE441_SHA 0.3 NEW_ASSESSMENT
```

Se uno scarto supera il limite il secondo comando scrive comunque il rapporto
e restituisce codice 2. Una prova tecnica a 0,25″/giorno ha rilevato quattro
superamenti e codice 2: **0,25 non è la soglia scelta dall'utente**. Errori
di input o copertura restituiscono errore invece di certificare dati parziali.

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Rapporto di confronto: `98097aa5f22a36712b0ec47d00c0a96373f72b626375da795ae26a1c75a1eedf`.
- Valutazione a 0,3″/giorno: `15dfd7f4d97dc2269fb948cc543bd775a7734f9ac77311c3642a41c9867f84d3`.

Due esecuzioni del confronto hanno prodotto lo stesso hash. Gli hash
identificano i file locali, non l'autenticità delle fonti esterne.
