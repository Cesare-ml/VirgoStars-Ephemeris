# VirgoStars Ephemeris — sintesi italiana

**Stato: primo prototipo isolato realizzato; accuratezza astronomica su kernel JPL reali NON ancora verificata.**

Il repository pubblico `virgostars-ephemeris` svilupperà da zero, con sorgenti originali e licenza MIT per il nostro codice, un motore di effemeridi astronomiche che legge i kernel NASA/JPL SPK senza usare Swiss Ephemeris.

La parola «equivalente» rappresenta **l'obiettivo di copertura delle funzioni necessarie** a VirgoStars: non significa che esista già un sostituto, né che tutti gli algoritmi/risultati Swiss saranno identici. La precisione va verificata con dati indipendenti (JPL Horizons/NAIF), tenendo conto di modello, epoca, coordinate e correzioni utilizzate.

Per ora il prototipo legge dati SPK di tipo 2, calcola posizioni e velocità geometriche J2000 e include una prima modalità Analytic Lite senza file esterni (JPL approssimato/USNO). Sono stati superati 18 test locali con dati sintetici e test dello strumento Horizons: non sono stati effettuati confronti con kernel NASA reali o Horizons; esiste ora lo script manuale `npm run verify:horizons`. Il motore completo dovrà fornire tempi astronomici, posizioni, velocità e coordinate; `astrological-functions` si occuperà di case, aspetti, temi, transiti, sinodi, rivoluzioni e altri metodi astrologici. Il sito e l'Astrolabio continueranno a usare il motore attuale fino a un collaudo separato.

**Dati e diritti:** i kernel distribuiti da NAIF sono dati di terzi con proprie condizioni, non diventano MIT. Il loro download/uso/redistribuzione sono disciplinati dalle regole NAIF; il toolkit SPICE ha restrizioni specifiche. Non inserire file di effemeridi, codice Swiss, segreti o dati privati nel repository.

Documenti principali: [architettura](ARCHITECTURE.md), [contratti](API_CONTRACTS.md), [roadmap](ROADMAP.md), [verifiche](VALIDATION.md), [dati e licenze](DATA_AND_LICENSING.md).
