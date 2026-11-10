# VirgoStars Ephemeris — sintesi italiana

**Stato: primo prototipo isolato realizzato; accuratezza astronomica su kernel JPL reali NON ancora verificata.**

Il repository pubblico `virgostars-ephemeris` sviluppa un motore originale di effemeridi astronomiche, con codice originale sotto **CPAL-1.0** (attribuzione VirgoStars secondo LICENSE e NOTICE), senza usare codice Swiss Ephemeris. **Rust è il solo motore astronomico canonico**: su `main` sono presenti prototipi JavaScript sperimentali; il workspace Rust e il lettore binario classico DE441 sono in sviluppo su `dev/ephemeris`, in attesa di integrazione verificata. I binding WASM/TypeScript e Python sono previsti, non ancora disponibili.

La parola «equivalente» rappresenta **l'obiettivo di copertura delle funzioni necessarie** a VirgoStars: non significa che esista già un sostituto, né che tutti gli algoritmi/risultati Swiss saranno identici. La precisione va verificata con dati indipendenti (JPL Horizons/NAIF), tenendo conto di modello, epoca, coordinate e correzioni utilizzate.

Per ora il prototipo legge dati SPK di tipo 2, calcola posizioni e velocità geometriche J2000 e include una prima modalità Analytic Lite senza file esterni (JPL approssimato/USNO). Sono stati superati 18 test locali con dati sintetici e test dello strumento Horizons: non sono stati effettuati confronti con kernel NASA reali o Horizons; esiste ora lo script manuale `npm run verify:horizons`. Il motore completo dovrà fornire tempi astronomici, posizioni, velocità e coordinate; `astrological-functions` si occuperà di case, aspetti, temi, transiti, sinodi, rivoluzioni e altri metodi astrologici. Il sito e l'Astrolabio continueranno a usare il motore attuale fino a un collaudo separato.

**Dati e diritti:** i kernel distribuiti da NAIF sono dati di terzi con proprie condizioni, non diventano CPAL-1.0. Il loro download/uso/redistribuzione sono disciplinati dalle regole NAIF; il toolkit SPICE ha restrizioni specifiche. Non inserire file di effemeridi, codice Swiss, segreti o dati privati nel repository.

Documenti principali: [architettura](ARCHITECTURE.md), [ADR Rust](ADR-0001-RUST-CORE-AND-BINDINGS.md), [contratti](API_CONTRACTS.md), [roadmap](ROADMAP.md), [verifiche](VALIDATION.md), [dati e licenze](DATA_AND_LICENSING.md), [licenza e crediti](LICENSING_AND_CREDITS.md), [stato dei rami](BRANCH_INTEGRATION_STATUS_2026-11-10.md).


## Avanzamento: geometria J2000 e tempo della luce

La PR contiene ora `projectGeometricJ2000` per ottenere ascensione retta, declinazione e longitudine/latitudine **eclittiche J2000 approssimate**, con velocità; `solveOneWayLightTime` calcola iterativamente un vettore al tempo di emissione. Sono passati **5 nuovi test locali**; i 18 precedenti appartengono al checkpoint precedente e non sono stati rieseguiti insieme in questo turno.

**Non sono ancora posizioni astrologiche apparenti della data**: mancano le correzioni IAU complete e la validazione su kernel NASA reali. `astrological-functions` continuerà a rifiutare questi risultati come valori `APPARENT/ECLIPTIC_OF_DATE`.


## Precessione IAU 2006 — controllo indipendente

È stato aggiunto un modulo originale per l'obliquità media, i quattro angoli Fukushima–Williams, la matrice di precessione/bias IAU 2006 e le coordinate geometriche **medie della data**. I **5 nuovi test locali** superati includono i nove valori della matrice di riferimento ERFA, non solo casi sintetici. L'errore massimo per elemento osservato è di circa 1,1×10⁻¹⁶.

**Attenzione:** coordinate *medie* e coordinate *apparenti* non sono equivalenti. La nuova API non fornisce velocità nel sistema rotante della data perché è necessario considerare anche la derivata della matrice. Resta da realizzare e verificare la nutazione, l'aberrazione, le osservazioni topocentriche e il confronto con `.bsp` NASA autentici.


## Nutazione IAU 2000B completa — checkpoint

Implementati i **77 termini** della nutazione IAU 2000B con coordinate geometriche **vere della data**; aggiunta la composizione esplicita con la precessione IAU 2006 e la documentazione della licenza BSD dei coefficienti scientifici ERFA. Il benchmark matematico JavaScript riproduce i riferimenti ERFA con differenze dell'ordine di 10⁻²⁰ rad per le componenti e inferiori a 6×10⁻¹⁷ per la matrice.

**Attenzione:** sono stati aggiunti 5 casi di test Node, non ancora eseguiti nell'intera suite. Le coordinate vere della data non sono ancora posizioni planetarie *apparenti*: le correzioni dell'osservatore e i test reali NASA sono ancora mancanti.


## Verifica incrociata delle suite

Tutte e **sette le suite** presenti nel repository sono state valutate sul codice GitHub in un runtime JavaScript V8 isolato: **33 casi superati su 33**, inclusi lettura SPK sintetica, tipi 2/3, conversione dei tempi, calcoli analitici, coordinate, Horizons, precessione e nutazione. Per le API `Buffer`, `URL` e `node:test` sono stati utilizzati adattatori di test.

Questo risultato **non sostituisce** ancora l'esecuzione normale `npm test` con Node né la misura dell'accuratezza con `de440s.bsp` e Horizons. Il prodotto VirgoStars e gli Astrolabi non sono stati modificati.
