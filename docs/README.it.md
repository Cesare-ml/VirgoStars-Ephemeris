# VirgoStars Ephemeris — sintesi italiana

**Stato: primo prototipo isolato realizzato; accuratezza astronomica su kernel JPL reali NON ancora verificata.**

Il repository pubblico `virgostars-ephemeris` svilupperà da zero, con sorgenti originali e licenza MIT per il nostro codice, un motore di effemeridi astronomiche che legge i kernel NASA/JPL SPK senza usare Swiss Ephemeris.

La parola «equivalente» rappresenta **l'obiettivo di copertura delle funzioni necessarie** a VirgoStars: non significa che esista già un sostituto, né che tutti gli algoritmi/risultati Swiss saranno identici. La precisione va verificata con dati indipendenti (JPL Horizons/NAIF), tenendo conto di modello, epoca, coordinate e correzioni utilizzate.

Per ora il prototipo legge dati SPK di tipo 2, calcola posizioni e velocità geometriche J2000 e include una prima modalità Analytic Lite senza file esterni (JPL approssimato/USNO). Sono stati superati 18 test locali con dati sintetici e test dello strumento Horizons: non sono stati effettuati confronti con kernel NASA reali o Horizons; esiste ora lo script manuale `npm run verify:horizons`. Il motore completo dovrà fornire tempi astronomici, posizioni, velocità e coordinate; `astrological-functions` si occuperà di case, aspetti, temi, transiti, sinodi, rivoluzioni e altri metodi astrologici. Il sito e l'Astrolabio continueranno a usare il motore attuale fino a un collaudo separato.

**Dati e diritti:** i kernel distribuiti da NAIF sono dati di terzi con proprie condizioni, non diventano MIT. Il loro download/uso/redistribuzione sono disciplinati dalle regole NAIF; il toolkit SPICE ha restrizioni specifiche. Non inserire file di effemeridi, codice Swiss, segreti o dati privati nel repository.

Documenti principali: [architettura](ARCHITECTURE.md), [contratti](API_CONTRACTS.md), [roadmap](ROADMAP.md), [verifiche](VALIDATION.md), [dati e licenze](DATA_AND_LICENSING.md).


## Avanzamento: geometria J2000 e tempo della luce

La PR contiene ora `projectGeometricJ2000` per ottenere ascensione retta, declinazione e longitudine/latitudine **eclittiche J2000 approssimate**, con velocità; `solveOneWayLightTime` calcola iterativamente un vettore al tempo di emissione. Sono passati **5 nuovi test locali**; i 18 precedenti appartengono al checkpoint precedente e non sono stati rieseguiti insieme in questo turno.

**Non sono ancora posizioni astrologiche apparenti della data**: mancano le correzioni IAU complete e la validazione su kernel NASA reali. `astrological-functions` continuerà a rifiutare questi risultati come valori `APPARENT/ECLIPTIC_OF_DATE`.


## Precessione IAU 2006 — controllo indipendente

È stato aggiunto un modulo originale per l'obliquità media, i quattro angoli Fukushima–Williams, la matrice di precessione/bias IAU 2006 e le coordinate geometriche **medie della data**. I **5 nuovi test locali** superati includono i nove valori della matrice di riferimento ERFA, non solo casi sintetici. L'errore massimo per elemento osservato è di circa 1,1×10⁻¹⁶.

**Attenzione:** coordinate *medie* e coordinate *apparenti* non sono equivalenti. La nuova API non fornisce velocità nel sistema rotante della data perché è necessario considerare anche la derivata della matrice. Resta da realizzare e verificare la nutazione, l'aberrazione, le osservazioni topocentriche e il confronto con `.bsp` NASA autentici.
