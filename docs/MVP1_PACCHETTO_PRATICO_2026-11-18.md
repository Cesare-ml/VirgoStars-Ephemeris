# MVP 1: consegna pratica della libreria

**Stato: in sviluppo, non pubblicata.** Priorità: rendere utilizzabili insieme le funzionalità essenziali; affinamenti e confronti più estesi possono seguire. Il catalogo stellare e il server principale sono in pausa. Il nuovo percorso seguente non cambia automaticamente le API standard.

## Pianeti: accesso unico

`mvp_planets::planets_ut1` calcola uno o più corpi, oppure `MVP_PLANETS` per Sole, Luna e pianeti. Riusa il percorso lungo già verificato a campione e restituisce la stessa epoca e fonte per tutto l'insieme. Il file DE441, UT1, ΔT e la sua incertezza sono espliciti; non è una conversione dell'ora civile. Le velocità sono i candidati di compatibilità Swiss, non derivate esatte delle posizioni. Marte–Plutone indicano baricentri di sistema.

```rust,no_run
use virgostars_ephemeris_core::de441::De441;
use virgostars_ephemeris_core::mvp_planets::{planets_ut1, PlanetaryTime, MVP_PLANETS};

let mut eph = De441::open("de441.eph")?;
let time = PlanetaryTime {
    jd_ut1: 2_451_545.0,
    delta_t_seconds: 64.0,
    uncertainty_seconds: 2.0,
    source: "example estimate",
};
let result = planets_ut1(&mut eph, &MVP_PLANETS, time)?;
for position in result.positions {
    println!("{:?}: {} degrees", position.body, position.rates.base.longitude_deg);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

In alternativa si può scegliere esplicitamente `PlanetaryTime::espenak_meeus_2006(jd_ut1, uncertainty_seconds)`: usa il modello già presente per gli anni astronomici −1999–3000 e richiede comunque un'incertezza dichiarata. I valori dopo il 2005 sono proiezioni di quella pubblicazione, non osservazioni. Fuori da tale intervallo serve un valore ΔT fornito dal chiamante.

I valori di esempio non certificano il tempo storico. Un errore su un corpo interrompe la richiesta e identifica il corpo; non si restituisce un insieme parziale. È consentita una selezione non vuota, senza duplicati. Il confronto già registrato sulle posizioni e le velocità è campionato, non una garanzia sull'intero intervallo 13000 a.C.–3999 d.C.

## Nodi, case e alba/tramonto: stesso tempo esplicito

Per un consumatore Rust, `mvp_snapshot::snapshot_ut1` riunisce pianeti, punti lunari e angoli delle case per un sito, con ricerca facoltativa di alba e tramonto. I risultati del sito sono separati e possono contenere un errore o indicare l'assenza dell'evento: il chiamante deve controllarli. Non fornisce le cuspidi astrologiche e non converte l'ora civile in UT1.

- `mvp_lunar::lunar_points_ut1` usa lo stesso `PlanetaryTime` per nodo medio, nodo vero, apogeo medio e apogeo osculante, con coordinate eclittiche ed equatoriali. Per le epoche remote usa esplicitamente il modello sperimentale di orientamento: i polinomi lunari e le velocità non sono ancora stati verificati in modo indipendente su tale intervallo. Il percorso moderno `lunar_point` mantiene i suoi limiti.
- `mvp_site::house_angles_ut1` restituisce gli angoli astronomici per le case; le cuspidi restano in `astrological-functions`. `experimental_house_angles_ut1` seleziona esplicitamente l'orientamento esteso.
- `mvp_site::solar_event_ut1` calcola alba o tramonto con disco superiore, parallasse e rifrazione secondo il motore esistente; un risultato assente indica che non c'è evento nella finestra richiesta. ΔT viene mantenuto costante durante la ricerca. `experimental_solar_event_ut1` seleziona l'orientamento esteso.

Le funzioni predefinite mantengono il limite moderno; `mvp_snapshot::experimental_snapshot_ut1` seleziona esplicitamente l'estensione per case e orizzonte. Il nuovo orientamento usa la longitudine media della Terra di Simon et al. (1994) proiettata sulla precessione di lungo periodo Vondrák et al. (2011), con ΔT esplicito. È una stima estrapolata, non una garanzia di precisione fisica. Su 14 date e orari campione, il tempo siderale differisce da Swiss 2.09.03 di meno di 10″; su quattro date campione ascendente e Medio Cielo di meno di 20″. Su quattro date a Roma alba e tramonto differiscono da Swiss meno di un secondo, con lo stesso ΔT iniziale e DE441: questi confronti non dimostrano l'intero intervallo. Su quattro date, nodo vero e apogeo osculante differiscono in longitudine da Swiss meno di 1″; nella data più antica il nodo medio differisce di circa 0,98° e l'apogeo medio di circa 0,41°, quindi questi due modelli richiedono ancora lavoro. Le API non convertono ora civile o UTC in UT1.

## Mancano per un primo pacchetto completo

Sono già presenti nodi e apogei, angoli per le case e la ricerca di alba/tramonto; il binding Node espone già `riseTransUt`. I relativi percorsi di orientamento hanno limiti moderni: la disponibilità di queste funzioni non dimostra la copertura antica richiesta.

1. Collegare il percorso planetario e le scelte di tempo a un'interfaccia di consumo coerente, con esempi e configurazione dei file. Non fare download impliciti.
2. Integrare l'orientamento storico sperimentale con i sistemi di case e controllare i nodi e apogei medi antichi. Le cuspidi astrologiche restano in `astrological-functions`.
3. Integrare alba e tramonto storici nel pacchetto di consumo, mantenendo espliciti i casi senza evento e i limiti dei campioni verificati.
4. Definire un percorso pratico per l'ora civile storica e una fonte ΔT disponibile o scelta esplicitamente; nessuna conversione automatica inventata per date antiche.
5. Verificare il pacchetto end-to-end con i dati locali e documentare il perimetro realmente supportato, poi predisporre la distribuzione autorizzata.

La precisione storica fisica e ulteriori raffinamenti numerici sono attività successive; non vanno confusi con la disponibilità funzionale.
