# VirgoStars Ephemeris — sintesi italiana

**Stato: progettazione e documentazione; nessun motore pronto o verificato.**

Il repository pubblico `virgostars-ephemeris` svilupperà da zero, con sorgenti originali e licenza MIT per il nostro codice, un motore di effemeridi astronomiche che legge i kernel NASA/JPL SPK senza usare Swiss Ephemeris.

La parola «equivalente» rappresenta **l'obiettivo di copertura delle funzioni necessarie** a VirgoStars: non significa che esista già un sostituto, né che tutti gli algoritmi/risultati Swiss saranno identici. La precisione va verificata con dati indipendenti (JPL Horizons/NAIF), tenendo conto di modello, epoca, coordinate e correzioni utilizzate.

Questo motore fornirà tempi astronomici, posizioni, velocità e coordinate; `astrological-functions` si occuperà di case, aspetti, temi, transiti, sinodi, rivoluzioni e altri metodi astrologici. Il sito e l'Astrolabio continueranno a usare il motore attuale fino a un collaudo separato.

**Dati e diritti:** i kernel distribuiti da NAIF sono dati di terzi con proprie condizioni, non diventano MIT. Il loro download/uso/redistribuzione sono disciplinati dalle regole NAIF; il toolkit SPICE ha restrizioni specifiche. Non inserire file di effemeridi, codice Swiss, segreti o dati privati nel repository.

Documenti principali: [architettura](ARCHITECTURE.md), [contratti](API_CONTRACTS.md), [roadmap](ROADMAP.md), [verifiche](VALIDATION.md), [dati e licenze](DATA_AND_LICENSING.md).
