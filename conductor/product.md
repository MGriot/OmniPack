# OmniPack-Hybrid (Cross-Platform Ultra-Performance)

## 1. Visione del Prodotto
Sviluppare un motore di ottimizzazione spaziale 3D capace di adattarsi all'hardware ospite.
- **Su Mobile:** Esecuzione rapida tramite core Legacy ottimizzato (Level 1).
- **Su PC:** Calcolo parallelo estremo (Level 2) e Ricerca Intelligente (MCTS/Genetic) per gestire migliaia di oggetti in volumi complessi, utilizzando algoritmi di Machine Learning e Simulazione Fisica Realistica (Gravity & Friction).

## 2. Architettura di Calcolo Scalabile

### A. Livello 1: Motore Legacy (Deterministico)
- **Algoritmo:** Extreme Points (EP) con euristica Best-Fit Decreasing.
- **Scaling:** Esecuzione single-thread ottimizzata per risparmio energetico e compatibilità browser.

### B. Livello 2: Motore ML & Meta-Euristiche (Auto-Apprendimento)
- **Numba JIT Acceleration:** Parallelizzazione estrema per collision detection.
- **Deep Reinforcement Learning (DRL) / MCTS:** Simulazione di scenari futuri per minimizzare lo spazio sprecato.
- **Algoritmi Genetici:** Evoluzione delle sequenze di carico per ottimizzazione globale.

## 3. Gestione Forme Irregolari e Volumi Variabili
- **Voxelization Dinamica:**
  - Mobile: Griglia a bassa risoluzione (5cm).
  - PC: Griglia ad alta risoluzione (2mm).
- **Coordinate System:** WxHxD (X, Y, Z) con Y come asse verticale per coerenza con gli standard industriali e di rendering.

## 4. Specifiche Tecniche per Piattaforma
| Feature | Versione Mobile | Versione PC (Workstation) |
| :--- | :--- | :--- |
| Core Engine | Python (Optimized) | Python (Numba JIT / Parallel) |
| Search | Heuristic (Best-Fit) | MCTS / Genetic / ML |
| Rendering | Babylon.js (Low-Poly) | Babylon.js (High-Fidelity) |
| Stabilità | Static Check (CoG) | Dynamic Physics Simulation |

## 5. Requisiti di Stabilità e Sicurezza (Advanced)
- **Simulatore Fisico Integrato:** Analisi delle forze, verifica ribaltamento, e vincoli di carico massimo (Recursive Load Transfer).

## 8. Definizione Tecnologica
- **Stack:** Python 3.11+, FastAPI, Numba, NumPy, Babylon.js.
- **Connectivity:** REST API over Localhost/127.0.0.1.
