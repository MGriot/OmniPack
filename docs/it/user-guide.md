# Guida utente

*[English version](../user-guide.md)*

Questa guida descrive l'app OmniPack per Windows e Android. Le schermate sono le stesse;
sul telefono sono divise nelle schede **Configura**, **Vista 3D** e **Risultati**, con il
pulsante **Calcola** in basso.

L'app ha due modalità, che si scelgono nella barra degli strumenti:
- **Auto:** OmniPack posiziona tutto.
- **Manuale:** posizioni tu i colli, e OmniPack può completare il resto (vedi
  [Posizionamento manuale](#8-posizionamento-manuale)).

### Lingua

L'app è in italiano e in inglese.
- Al primo avvio usa la lingua del sistema: italiano se Windows o Android è in italiano,
  altrimenti inglese.
- Il selettore **EN / IT** all'estremità destra della barra degli strumenti cambia lingua
  in qualsiasi momento, e la scelta viene ricordata.
- In italiano numeri e date usano il formato italiano (0,45 e 1.200); in inglese
  usano il formato del sistema.
- Configurazioni, piani e soluzioni salvati non dipendono dalla lingua. L'API resta in
  inglese.
- Anche il programma di installazione per Windows (`setup.exe`) usa l'italiano su un
  sistema in italiano.

### Su telefono o tablet

L'app per telefono ha lo stesso motore e le stesse funzioni, con queste differenze:
- Un pannello alla volta: **Configura**, **Vista 3D** o **Risultati**, scelti in basso. I
  messaggi compaiono su una riga subito sopra queste schede.
- La barra degli strumenti scorre di lato. Prima vengono **Auto | Manuale** e
  **Soluzioni…**, poi Esempio, Nuovo, Apri, Salva, il catalogo e il selettore della lingua.
- In modalità manuale si tocca per posizionare. Non c'è l'anteprima al passaggio del
  mouse, quindi la sagoma verde o rossa compare solo mentre trascini. Per spostare un
  collo, scegli **Seleziona / sposta** nella barra sopra la vista 3D, poi trascinalo. I
  pulsanti ⟳, ↶ e ✕ della barra sostituiscono le scorciatoie da tastiera, e i campi esatti
  X / Y / Z sono in **Risultati**.
- Il pannello non si ridimensiona e non c'è l'API locale (è una funzione per desktop).

<img src="../images/android-manual.png" alt="Modalità manuale su telefono: tre pallet posizionati toccando lo schermo, la barra di posizionamento sopra la vista 3D e i colli ancora da posizionare" width="300">

Le immagini di questa guida mostrano l'app in inglese; i comandi sono negli stessi punti.

### Il pannello di configurazione

Il pannello di configurazione a sinistra ha quattro schede: **Container**, **Carico**,
**Strategia** e **Fisica**. In modalità manuale ce n'è una quinta, **Posiziona**.
- In ogni scheda le impostazioni sono raggruppate in sezioni che si possono chiudere.
- Una sezione chiusa mostra un riepilogo su una riga, per esempio "40 piedi high cube ·
  2.352 × 2.698 × 12.032 mm · porta 2.340 × 2.585".
- L'app ricorda la scheda aperta, quali sezioni sono aperte e la larghezza del pannello.
- Per allargare o restringere il pannello, trascinane il bordo destro (desktop).

## 1. Descrivi il container

Nella scheda **Container**, scegli un **Tipo** oppure inserisci tu larghezza (X), altezza
(Y) e profondità (Z) interne in millimetri. **La porta è in fondo all'asse della
profondità** ed è disegnata in arancione. La profondità 0 è la parete frontale, contro cui
la frenata spinge il carico.

| Tipo | Interno largh. × alt. × prof. (mm) | Porta (mm) | Tara (kg) | Portata (kg) |
|---|---|---|---|---|
| 20 piedi standard (20' DV) | 2352 × 2393 × 5898 | 2340 × 2280 | 2230 | 28 250 |
| 40 piedi standard (40' DV) | 2352 × 2393 × 12 032 | 2340 × 2280 | 3750 | 26 730 |
| 40 piedi high cube (40' HC) | 2352 × 2698 × 12 032 | 2340 × 2585 | 3900 | 26 580 |
| Semirimorchio 13,6 m | 2480 × 2700 × 13 600 | carico laterale | — | 24 000 |

Sono valori tipici per una massa lorda massima di 30 480 kg; controlla la targa CSC del
container reale. Per i container ISO viene impostata anche una portata del pianale di
2500 kg/m².

Limiti facoltativi:
- **Portata utile max** (kg).
- **Scostamento max baricentro:** di quanto il baricentro del carico può spostarsi dalla
  linea mediana, in mm. È un limite rigido; i controlli di bilanciamento del Codice CTU
  descritti sotto sono avvisi.
- **Larghezza / altezza porta:** ogni collo deve passare dalla porta nella posizione in
  cui è caricato. Un collo troppo alto in piedi viene coricato, se può essere ruotato;
  altrimenti resta fuori con il motivo "porta troppo piccola".
- **Tara** (kg): dà la **VGM** (massa lorda verificata = tara + carico, SOLAS) e conta
  nei carichi sugli assi del veicolo.
- **Portata pianale** (kg/m²): i colli che premono di più sul pianale vengono segnalati,
  perché il loro carico va ripartito con travetti.

Lascia un campo vuoto per "nessun limite".

**Veicolo stradale** (una sezione a parte): scegli un trattore + telaio portacontainer a
3 assi per vedere asse sterzante, asse motore, assi del semirimorchio e massa complessiva
rispetto ai limiti UE (10 t, 11,5 t, 24 t, 44 t). **Geometria e limiti del veicolo**
permette di inserire il tuo veicolo: distanze lungo il veicolo in mm, positive verso il
retro, misurate dal perno ralla o dall'asse sterzante.

## 2. Aggiungi il carico

La scheda **Carico** ha una riga per ogni tipo di articolo: colore, nome, forma e
dimensioni, quantità e massa per collo.
- Fai clic su una riga per aprire la scheda completa, con tutti i campi e un'anteprima
  3D. Un altro clic la chiude.
- **⧉** duplica un articolo e **✕** lo rimuove.
- Con più di sei articoli compare un filtro per cercarli per nome.

Usa **+ Aggiungi articolo…** per aggiungere un tipo di articolo, scegliendone la forma:

| Forma | Campi delle dimensioni | Carico tipico |
|---|---|---|
| Scatola | largh. × alt. × prof. | cartoni, casse, pallet |
| Cilindro / fusto | raggio, lunghezza | fusti, bobine, tubi |
| Sfera | raggio | palle, serbatoi |
| Cono | raggio base, altezza | coni stradali, tramogge |
| Piramide | largh. × prof. della base, altezza | piramidi imballate, cappe |
| Prisma (n lati) | lati (3 = triangolo, 6 = esagono…), raggio, lunghezza | travi, barre, estrusi |
| Profilo a L (angolare) | lato A, lato B, spessore, lunghezza | angolari in acciaio |

**+ Aggiungi articolo…** offre anche pallet carichi: **Europallet EPAL 1** (1200 × 800) e
**Pallet industriale EPAL 2** (1200 × 1000). Entrambi partono alti 1 m, 500 kg, con
"Questo lato in alto" e attrito 0,45 (legno su compensato); imposta altezza e massa reali.
In un container largo 2352 mm due EPAL 1 non stanno affiancati sui lati lunghi (2400 mm).
La rotazione permette a OmniPack di alternare gli orientamenti sulla larghezza
(1200 + 800, lo schema a girandola). Due EPAL 2 stanno affiancati sui lati da 1000 mm.

Ogni scheda articolo mostra un'**anteprima 3D**: trascinala per ruotarla. Mostra il
parallelepipedo d'ingombro, gli assi X/Y/Z e il baricentro.

Per ogni tipo di articolo puoi impostare:
- **Massa** (kg per collo) e **Quantità**.
- **Carico max sovrapposto** (kg): conta tutto ciò che è impilato sopra, non solo il
  collo appoggiato direttamente. **Fragile** significa che sopra non può andare niente.
- **Tappa:** la tappa di consegna, dove 1 viene scaricata per prima. Lascia 0 per il
  carico che resta a bordo.
- **Zona:** ovunque, in fondo (verso la parete frontale) o vicino alla porta.
- **Attrito μ:** l'attrito contro ciò su cui l'articolo appoggia. Lascialo vuoto per usare
  il valore predefinito.
- **Questo lato in alto / Solo in piedi:** mantiene verticale l'asse dell'altezza.
- **Solo a terra:** l'articolo deve stare sul pianale del container.
- **Baricentro:** X / Y / Z in mm dall'angolo in basso a sinistra sul retro
  dell'articolo non ruotato. Lascia vuoti i campi per il centro geometrico, oppure usa ↺
  per tornarci.

Gli esempi (il menu **Esempio…**) mostrano configurazioni complete: un carico misto su
camion, tutte le forme in un container da 20 piedi e alcuni benchmark.

## 3. Scegli la strategia di carico

Queste impostazioni sono nella scheda **Strategia**.

- **Ordine di scarico:**
  - **LIFO:** per veicoli scaricati da una porta posteriore. L'ultima tappa viene
    caricata per prima, in fondo, e la prima tappa finisce vicino alla porta.
  - **FIFO:** per il carico laterale o passante. La prima tappa viene caricata per prima
    e il riempimento parte dalla porta.
- **In ogni tappa, carica:** prima i più grandi, i più pesanti, quelli con la base più
  ampia o i più alti, oppure come in elenco.
- **Schema di riempimento:** l'ordine in cui viene usato lo spazio. Scegli pareti sulla
  larghezza, strati a terra completi, pareti sulla lunghezza, file sulla lunghezza, oppure
  la crescita da un angolo.
- **Appreso (dai tuoi piani salvati):** compare dopo che hai addestrato un modello (vedi
  [Soluzioni salvate e posizionamento appreso](#9-soluzioni-salvate-e-posizionamento-appreso)).
  Posiziona i colli come fanno i piani che hai segnato.
- **★ Migliore: prova tutti gli schemi e gli ordini:** invece di un solo schema,
  OmniPack cerca. Prova ogni schema di riempimento e ogni priorità di carico, poi fa
  evolvere ordini di carico e orientamenti (una ricerca genetica seguita da una ricerca
  locale). Ogni candidato riceve il controllo fisico completo; tappe, zone e vincoli
  "solo a terra" vengono sempre rispettati.
  - **Tempo di ricerca** (5–120 s): la ricerca tiene i piani migliori trovati in quel
    tempo. Mentre è in corso, **Calcola** diventa **Ferma ■**; fermandola si tiene quanto
    trovato fino a quel momento.
  - **Preferisci:** sposta il cursore verso *densità max* per riempire più volume, o
    verso *meno fissaggi* per favorire i piani che richiedono meno ancoraggi e
    riempitivi.
  - In **Risultati** puoi scegliere fra un massimo di tre piani ben diversi tra loro,
    accanto a quello che darebbero le tue impostazioni.
- **Margine di stabilità:** quanto ogni baricentro deve restare all'interno della sua
  area di appoggio. 0 significa "non si ribalta per poco"; valori più alti sono più
  sicuri.
- **Area di appoggio minima:** la quota di un fondo piatto che deve poggiare su qualcosa.
- **Equilibrio:** quanto tenere il carico centrato tra sinistra e destra.
- **Consenti la rotazione:** permette agli articoli di girarsi in ognuno dei loro
  orientamenti di appoggio.
- **Controlli di bilanciamento del Codice CTU:** avvisa quando il carico è distribuito
  male (vedi [Bilanciamento del carico](#bilanciamento-del-carico) più sotto). I cursori
  sotto la casella impostano le soglie:
  - lo scostamento del baricentro (±5 % della lunghezza e della larghezza);
  - la massa minima nella metà centrale della lunghezza (60 %);
  - l'altezza massima del baricentro (50 %).
- **Centra il carico in lunghezza:** dopo il calcolo, fa scorrere un carico parziale
  lungo la lunghezza, del minimo necessario per rispettare la finestra del baricentro e i
  limiti sugli assi. I vuoti lasciati alla parete frontale e alla porta vanno poi
  puntellati con legname o cuscini gonfiabili. Disattivato per impostazione predefinita.

## 4. Scegli la fisica

Appoggio sotto gravità, ribaltamento a riposo, limiti di impilamento e portata vengono
sempre verificati. La scheda **Fisica** ha queste sezioni:

- **Tratte di trasporto:** spunta ogni tratta del viaggio, per esempio Strada, poi
  Ferrovia (smistamento), poi Mare zona B. Ogni tratta predefinita riporta le sue
  accelerazioni (avanti / indietro / laterale, in g).
- **Scivolamento:** l'attrito deve trattenere l'articolo, a meno che una parete o una
  catena di colli a contatto lo blocchi.
- **Ribaltamento:** l'articolo e tutto ciò che porta non devono ribaltarsi, a meno che
  siano bloccati più in alto del baricentro.
- **Evita il ribaltamento:** non posiziona mai un collo dove si ribalterebbe durante il
  trasporto (per esempio corica gli articoli snelli). Un collo che non trova altro posto
  viene comunque caricato e segnalato per l'ancoraggio.
- **Impilamento dinamico:** i limiti di impilamento vengono verificati aggiungendo
  l'accelerazione verticale, il che conta in mare.
- **Cunei per articoli tondi:** fusti coricati e sfere sono tenuti con cunei. Disattivalo
  per richiedere invece che siano incastrati da pareti o colli vicini.
- **Fine carico bloccata:** una barra di bloccaggio, una sponda o dei riempitivi chiudono
  l'estremità aperta del carico.
- **Tappetini antiscivolo:** tappetini in gomma sotto ogni collo e tra gli strati
  (μ ≥ 0,6). Su strada reggono 0,5 g laterali, ma non 0,8 g in frenata.
- **Carico sul pianale:** valori di attrito tipici da EN 12195-1 (pallet in legno segato
  su compensato 0,45, pallet in plastica 0,2, …). Sceglierne uno imposta
  **Attrito predefinito μ**.
- **I riempitivi colmano vuoti fino a** (mm): i vuoti fino a questa misura tra colli, o
  tra un collo e una parete, contano come riempiti con riempitivi o cuscini gonfiabili e
  bloccano come un contatto diretto. 0 significa che bloccano solo le facce a contatto.
  Con il comune μ 0,4 l'attrito da solo non trattiene mai il carico su strada, quindi
  quasi tutto dipende dal bloccaggio. Se questo valore è troppo piccolo per la tua
  pratica, quasi ogni collo risulterà da ancorare.
- **Ancoraggio diretto (EN 12195-1):** la capacità di ancoraggio LC della cinghia
  (predefinita 2000 daN), la resistenza dei punti di ancoraggio (punti a pavimento dei
  container: 1000 daN) e gli angoli di ancoraggio. Ogni forza di fissaggio viene allora
  data anche come numero di ancoraggi. Conta il più debole tra cinghia e punto di
  ancoraggio.
- **Caso marittimo dai moti della nave:** inserisci larghezza e GM della nave, le
  ampiezze di rollio e beccheggio, il periodo di beccheggio e la posizione del container,
  cioè l'altezza sull'asse di rollio e la distanza dalla sezione maestra. OmniPack calcola
  il periodo di rollio e le accelerazioni in quel punto. **Aggiungi questo caso** mette il
  caso nell'elenco delle tratte (✕ lo rimuove). Il risultato avvisa se la nave è
  **rigida** (periodo di rollio breve, accelerazioni violente sul ponte) o **poco
  stabile** (GM sotto 0,15 m).

## 5. Calcola e leggi i risultati

Premi **Calcola** (o Ctrl+Invio). I badge in cima a **Risultati** riassumono il piano:

- **✓ Stabile a riposo**, oppure **✗ Violazioni trovate** con l'elenco dei problemi.
- **Fissaggio:** **✓ Fissato per il trasporto**, oppure **⚠ N colli da ancorare**. Ogni
  tratta di trasporto scelta elenca allora cosa scivolerebbe o si ribalterebbe, in quale
  direzione, e la forza (kN) che l'ancoraggio o il bloccaggio deve fornire. Le forze più
  alte vengono prima. **Riempi N vuoti con riempitivi** elenca i vuoti su cui conta il
  bloccaggio, con la loro larghezza.
- **Cunei:** quanti articoli tondi richiedono cunei.
- **✓ Bilanciato** oppure **⚠ N avvisi di bilanciamento**, e **N oltre la portata del
  pianale** quando dei colli premono troppo sul pianale.

Ogni riga di fissaggio dà la forza di bloccaggio e il numero di ancoraggi diretti, per
esempio "bloccare con ≥ 1,32 kN o 1 ancoraggio diretto (1.000 daN)". Nell'elenco dei
riempitivi, i vuoti da 150 mm in su sono indicati con "cuscino gonfiabile".

Sotto i badge trovi il riempimento in %, la massa del carico, il baricentro, i carichi
sugli assi, l'accessibilità allo scarico (la quota di colli raggiungibili alla loro tappa
senza spostare il carico di un'altra tappa) e gli eventuali colli che non è stato
possibile caricare, con il motivo.

### Bilanciamento del carico

La sezione **Bilanciamento del carico** verifica il carico secondo il Codice CTU
(✓ / ⚠):

- **Baricentro in lunghezza / laterale:** quanto il baricentro del carico dista dal
  centro, in mm e come quota della lunghezza o della larghezza (limite ±5 %).
- **Metà centrale:** la quota di massa tra il 25 % e il 75 % della lunghezza (almeno il
  60 %). Un container riempito in modo uniforme ne ha il 50 %, quindi porta i colli
  pesanti verso il centro.
- **Altezza del baricentro:** sotto metà dell'altezza interna.
- **Metà frontale / metà porta:** dove si trova la massa.
- **Spostato in lunghezza:** di quanto "Centra il carico in lunghezza" ha fatto scorrere
  il carico.
- **Spazio libero fronte / porta:** "puntellare" segna un vuoto più grande del limite dei
  riempitivi.
- **VGM:** tara + carico, a cui vanno aggiunti riempitivi e materiale di ancoraggio.
- **Asse sterzante, asse motore, assi del semirimorchio, massa complessiva** rispetto ai
  loro limiti, quando è impostato un veicolo stradale.
- I colli **oltre la portata del pianale**, ciascuno con l'area su cui va ripartito il
  suo carico.

Sono avvisi: il piano resta valido. **★ Migliore** preferisce i piani con avvisi di
bilanciamento meno numerosi e più piccoli; il cursore **Preferisci** dà loro più peso
verso *meno fissaggi*.

## 6. Esplora la vista 3D

- **Ruota** trascinando con il tasto sinistro o con un dito, **ingrandisci** con la rotella
  o con due dita, **sposta** trascinando con il tasto destro. ⟲ ripristina la vista.
- **Colora per** articolo, tappa di consegna, carico rispetto al limite, margine di
  stabilità, fissaggio necessario, mappa delle sollecitazioni o pressione sul pianale.
  - *Fissaggio necessario:* rosso = da ancorare, viola = pila sovraccarica, arancione =
    cunei, blu = stabile una volta riempiti i vuoti elencati, verde = fissato.
  - *Mappa delle sollecitazioni:* la forza di trasporto che ogni collo deve trasmettere a
    ciò che lo blocca, nella tratta e nella direzione peggiori. È la sua spinta più tutto
    ciò che sta dietro nella catena di bloccaggio. Verde è basso, rosso è il valore più
    alto in questo container. Le quattro fasce si possono isolare come ogni voce della
    legenda.
  - *Pressione sul pianale:* la pressione sul pianale rispetto alla sua portata. Viola
    significa oltre la portata; i colli grigi non poggiano sul pianale.
- Con i controlli del Codice CTU attivi, il pianale mostra la finestra consentita per il
  baricentro del carico (verde quando l'indicatore rosa del baricentro è dentro, rossa
  quando non lo è). Le linee tratteggiate segnano il 25 % e il 75 % della lunghezza.
- **Legenda:** fai clic sulle voci per mostrare solo quei gruppi (gli altri si
  attenuano); un altro clic ne toglie uno; **Mostra tutto** ripristina.
- **Sequenza di carico:** usa ⏮ ◀ ▶ ▶| ⏭ o i tasti ← → Home Fine Spazio per scorrere il
  carico un collo alla volta. Il collo successivo appare come sagoma arancione nella sua
  posizione, e nome, massa e coordinate compaiono sotto il cursore.
- **Fai clic su un collo** per vederne i dettagli: ordine di carico, orientamento, carico
  sovrapposto, margine di stabilità ed eventuale fissaggio necessario.

## 7. Salva ed esporta

- **Salva… / Apri…:** l'intera configurazione come file JSON.
- **Catalogo:** configurazioni con nome conservate nell'app.
- **Esporta piano (JSON):** ogni posizionamento con le sue coordinate, per altri
  programmi.
- **Esporta lista di carico (CSV):** una lista di carico per il magazzino, con posizione,
  dimensioni, orientamento, massa, pressione sul pianale, tappa, cunei, ancoraggi e note
  di fissaggio. I nomi delle colonne restano in inglese in ogni lingua, così i programmi
  che leggono il file non cambiano; le note seguono la lingua dell'app.
- **Salva soluzione…:** conserva il piano, con la sua configurazione, nell'app (vedi
  sotto).

## 8. Posizionamento manuale

Passa a **Manuale** nella barra degli strumenti. La vista 3D mostra ora il tuo piano, e la
scheda **Posiziona** elenca ogni articolo con il numero di colli ancora da posizionare.

1. **Scegli un collo** nella scheda Posiziona, o nella barra sopra la vista 3D sul
   telefono.
2. **Scegli l'orientamento** con i pulsanti, che mostrano l'ingombro
   largh. × alt. × prof. nel container, oppure premi **R** per scorrerli. Sono proposti
   solo gli orientamenti consentiti dall'articolo.
3. **Punta** nella vista 3D. Una sagoma trasparente mostra dove finirebbe il collo:
   scende sul pianale o sui colli sottostanti.
   - Verde significa che la posizione supera le verifiche.
   - Rosso significa che non le supera, e la barra dice perché (per esempio "non a
     terra", "sovrapposizione", "instabile").
4. **Fai clic o tocca** per posizionarlo. Continua a fare clic per posizionare altri
   colli dello stesso articolo; **Esc** lo lascia.

Spostare e modificare:
- **Trascina** un collo posizionato per spostarlo. La vista non ruota mentre trascini.
- **Fai clic** su un collo per selezionarlo. Le frecce lo spostano di 10 mm, o di 100 mm
  con Maiusc. **Canc** lo rimuove.
- Il pannello **Risultati** ha i campi X / Y / Z per le posizioni esatte, più ⟳ Ruota e
  Rimuovi.
- **Ctrl+Z** annulla gli ultimi passi.

Opzioni:
- **Appoggia per gravità** (attiva): i colli poggiano sempre su ciò che sta sotto.
  Disattivala per mettere un collo all'altezza che indichi o digiti; un collo sospeso
  viene segnalato.
- **Aggancia a pareti e facce** (attiva): entro 30 mm da una parete o dalla faccia di un
  altro collo, il collo si allinea.

**Non viene rifiutato nulla.** Si può usare qualsiasi posizione. Il piano viene verificato
esattamente come uno automatico, e ogni problema compare in Violazioni: sovrapposizioni,
colli sospesi o instabili, sovraccarichi, la porta, i colli "solo a terra". Il badge
conta i problemi segnalati. Fissaggio per il trasporto, bilanciamento del carico e
pressione sul pianale funzionano come in modalità auto.

L'**ordine di carico** è l'ordine in cui hai posizionato i colli. Se quell'ordine non è
fisicamente possibile (hai messo un collo sospeso e poi ne hai fatto scorrere uno sotto),
i colli vengono caricati dal più basso.

**Completa automaticamente** (o **Completa ▶** nella barra degli strumenti) carica
attorno ai tuoi tutti i colli non ancora posizionati, con la strategia attuale. I tuoi
colli mantengono la loro posizione. Su un tuo collo che non supera le verifiche non viene
impilato nulla. Se il resto non entra, vengono aggiunti altri container. Puoi continuare a
modificare anche dopo.

Tornando ad **Auto** si rivede il piano automatico. Entrambi i piani vengono conservati.

## 9. Soluzioni salvate e posizionamento appreso

**Salva soluzione…** (sotto i risultati, in entrambe le modalità) conserva il piano
insieme alla sua configurazione. Spunta **Usa per l'addestramento** perché il piano
insegni a OmniPack il tuo modo di caricare. Solo i piani senza violazioni possono essere
usati per l'addestramento.

**Soluzioni…** nella barra degli strumenti elenca i piani salvati con:
- la data e il modo in cui è stato fatto ogni piano: auto, migliore, manuale o
  manuale + auto;
- se è valido (✓ / ✗);
- una casella **addestra**;
- **Apri**, che carica la configurazione e il piano (in modalità manuale per i piani
  fatti a mano), e **✕**, che lo elimina.

Sotto l'elenco:
- **Addestra modello** impara a valutare i posizionamenti dai piani segnati con
  "addestra". Riporta da quante decisioni di posizionamento ha imparato, e quanto spesso
  la posizione che hai scelto risulta prima: con lo schema di riempimento predefinito più
  vicino prima dell'addestramento, e con i pesi appresi dopo. Richiede un secondo o due.
- Lo schema di riempimento **Appreso (dai tuoi piani salvati)** compare allora nella
  scheda Strategia, e anche **★ Migliore** lo prova. **Azzera modello** lo dimentica.
- **Esporta dati di addestramento** scrive le decisioni come righe JSON, per addestrare
  altri modelli.

[Posizionamento appreso](../learning.md) (in inglese) spiega come funziona
l'apprendimento.

## 10. Collega altri sistemi (API)

Su Windows, **API…** nella barra degli strumenti permette ad altri programmi di usare
OmniPack mentre è aperto. Un sistema ERP come SAP, un sistema di magazzino o uno script
invia un container e il suo carico e riceve tutti i posizionamenti, in JSON o come lista
di carico CSV.

![La finestra API: l'API locale attiva sulla porta 8765 con una chiave generata](../images/desktop-api.png)

- **Attiva l'API** e **Applica** la avviano su `http://127.0.0.1:8765` (cambia la
  **Porta** se serve).
- **Chiave API:** i programmi devono inviarla come `X-API-Key`. **Nuova** ne crea una
  casuale; **Copia** la copia.
- **Consenti altri computer** ascolta sulla rete. Serve una chiave, e può essere
  necessario aprire la porta nel firewall di Windows.
- **Cartelle di scambio:** con una cartella di ingresso e una di uscita impostate, le
  richieste JSON o le liste di articoli CSV messe nella cartella di ingresso vengono
  pianificate, e il risultato e una lista di carico compaiono nella cartella di uscita.
- La riga di stato dice se l'API è attiva. Quando lo è, la finestra mostra un esempio di
  chiamata `curl`.

L'API si riavvia con l'app finché non la disattivi. Per un server senza l'app, usa
`omnipack-server`. [La guida all'API](../api.md) (in inglese) descrive gli endpoint, il
formato ERP semplice, job e callback, e l'integrazione con SAP.
