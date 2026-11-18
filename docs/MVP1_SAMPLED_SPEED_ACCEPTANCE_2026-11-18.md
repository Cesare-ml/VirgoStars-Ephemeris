# MVP 1: criteri campionati per le velocità

**I criteri scelti dall'utente sono superati nei rispettivi campioni.** Questo
non dichiara rilasciata la MVP: i campioni non sono un limite continuo, e
restano da integrare il consumer Desktop e il contratto delle date storiche.
Stelle e server principale rimangono in pausa.

Il riferimento è la velocità pubblicata da Swiss Ephemeris 2.09.03, usando
DE441 comune, posizioni apparenti geocentriche, eclittica mista direct-obliquity,
TDB per il motore e TT approssimato per Swiss. Le grandezze del motore sono
candidati sperimentali esplicitamente selezionati, non derivate delle posizioni
e non l'output predefinito. Mercurio e Venere sono centri planetari;
Marte–Plutone sono baricentri di sistema. Gli stessi dati orbitali non sono
una validazione indipendente dell'orbita o dell'accuratezza fisica storica.

| Controllo | Limite scelto | Campioni | Massimo osservato |
| --- | ---: | ---: | ---: |
| Longitudine della Luna | 0,3″/giorno | 194 istanti, inclusi gli estremi | 0,270162″/giorno |
| Altre longitudini e tutte le latitudini | 0,1″/giorno | 320 corpo/epoca, 608 componenti | 0,024042″/giorno |
| Velocità radiali | 0,0000005 UA/giorno | 320 corpo/epoca | 0,000000231740 UA/giorno |
| Cambi di moto | 15 s | 96 stazioni selezionate | 11,536 s |

I 320 casi angolari comprendono Sole, Luna, Mercurio, Venere e i sei
baricentri, per otto anni da −12500 a 3900 e quattro mesi; la longitudine
lunare è esclusa da questo controllo e ha il limite a sé. I 320 casi radiali
usano altre otto epoche da −11000 a 3800: la correzione radiale del Sole è
esplicitamente esclusa. Le griglie non coincidono e **non** attestano ogni
giorno fra 13000 a.C. e 3999 d.C. Le stazioni sono zeri numerici TDB e non
determinano l'orario civile antico. I rapporti specialistici conservano gli
ulteriori limiti, incluso il peggioramento di alcuni casi lunari.

## Valutazioni riproducibili

Gli strumenti riusano i rapporti locali, controllando hash, schema, matrice
completa e valori finiti. Le soglie sono sempre argomenti espliciti; un
superamento scrive il risultato e restituisce codice 2. In particolare, non
si applica la soglia angolare alla distanza né il limite di posizione di 1″
alle velocità. I file di riferimento e DE441 restano fuori dal repository.

```text
node bindings/node/scripts/assess-mvp1-angular-speeds.cjs ANGULAR ANGULAR_SHA DE441_SHA 0.1 NEW_REPORT
node bindings/node/scripts/assess-mvp1-radial-speeds.cjs RADIAL RADIAL_SHA DE441_SHA 0.0000005 NEW_REPORT
```

- Rapporto angolare: `43941ff2eb4b838ef37184472ff83d7050e4bf70a6e6a9003f47fda62ec7c69e`.
- Rapporto radiale: `d40d33dbdc46e9a75f16cb96d36bbb7958e3760a1308f5ee200f570a926c0efc`.
- DE441: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Valutazione angolare a 0,1″/giorno: `a9de8f65b54e1ed06b19d6ab9b39c90a07a1cd9bb33711c9f532f6f763bf0353`.
- Valutazione radiale a 0,0000005 UA/giorno: `0b8f34be789f8a0c01bad463c8b395de831caab9caa392c053b7478daa59c22f`.

Per verificare il comportamento negativo, prove tecniche più severe a
0,02″/giorno e 0,0000002 UA/giorno rilevano ciascuna due superamenti e
restituiscono codice 2. **Non sono le soglie scelte dall'utente.** Il controllo
lunare e quello delle stazioni sono documentati rispettivamente in
[velocità lunare](MVP1_LUNAR_LONGITUDE_SPEED_2026-11-18.md) e
[stazioni e UT1](MVP1_STATIONS_AND_HISTORICAL_RATES_2026-11-18.md).
