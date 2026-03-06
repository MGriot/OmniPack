# OmniPack-Hybrid (Cross-Platform Ultra-Performance)

## 1. Visione del Prodotto
Sviluppare un motore di ottimizzazione spaziale 3D capace di adattarsi all'hardware ospite e di gestire vincoli logistici complessi (FIFO, LIFO, Accessibilità).
- **Su Mobile:** Esecuzione rapida tramite core nativo (Rust/C++) o Python ottimizzato, con UI cross-platform.
- **Su PC:** Calcolo parallelo estremo (Numba/Rust), Ricerca Intelligente (MCTS/Genetic) e indicizzazione spaziale (R-Trees) per gestire migliaia di oggetti in volumi complessi.

## 2. Architettura di Calcolo Scalabile

### A. Livello 1: Motore Legacy (Deterministico)
- **Algoritmo:** Extreme Points (EP) con euristica Best-Fit Decreasing.
- **Scaling:** Esecuzione single-thread ottimizzata per risparmio energetico.

### B. Livello 2: Motore ML & Meta-Euristiche (Auto-Apprendimento)
- **Accelerazione Rust/Numba:** Migrazione dei loop critici (collision detection) in Rust per massime performance.
- **Indicizzazione Spaziale:** Utilizzo di R-Trees o Octrees per ridurre la complessità della ricerca da O(N²) a O(N log N).
- **Deep Reinforcement Learning (DRL) / MCTS:** Simulazione di scenari futuri per minimizzare lo spazio sprecato.
- **Algoritmi Genetici:** Evoluzione delle sequenze di carico con seeding euristico diversificato.

## 3. Vincoli Logistici e Accessibilità (New)
- **FIFO (First-In, First-Out):** Gli oggetti caricati per primi vengono posizionati sul fondo (back) del container.
- **LIFO (Last-In, First-Out):** Gli oggetti caricati per ultimi sono i primi ad essere scaricati e vengono posizionati vicino al portellone (front).
- **Accessibilità Dinamica:** Calcolo di un punteggio di accessibilità basato sulla posizione relativa al punto di scarico (door) e agli ostacoli (blocking items).
- **Priorità di Carico:** Gestione di gruppi di priorità per il carico sequenziale e il bilanciamento del peso (Center of Gravity).

## 4. Gestione Forme Irregolari e Volumi Variabili
- **Voxelization Dinamica:**
  - Mobile: Griglia a bassa risoluzione (5cm).
  - PC: Griglia ad alta risoluzione (2mm) con bitmasking per collision detection rapida.

## 5. Specifiche Tecniche per Piattaforma
| Feature | Versione Mobile | Versione PC (Workstation) |
| :--- | :--- | :--- |
| Core Engine | Rust (Shared Lib) / Python | Python (Numba JIT) / Rust |
| Spatial Index | Grid-based | R-Trees / Octrees |
| Search | Heuristic (GRASP) | MCTS / Genetic / ML |
| Rendering | Babylon.js / Native | Babylon.js (High-Fidelity) |
| Deployment | Capacitor / BeeWare | PyInstaller / Docker / Native EXE |

## 8. Definizione Tecnologica
- **Stack Core:** Python 3.11+, Rust (PyO3), Numba, NumPy.
- **Backend/API:** FastAPI, REST/WebSocket.
- **Frontend:** React + Babylon.js (3D Visualization).
- **Packaging:** Capacitor (Mobile), BeeWare/PyInstaller (Desktop).
