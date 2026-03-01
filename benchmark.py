import time
from core.models import Item, Container
from core.engine import Level1Engine
from core.engine_v2 import Level2Engine

def run_benchmark(num_items=50):
    container = Container("c1", 100, 100, 100)
    items = [Item(f"i{i}", 10, 10, 10) for i in range(num_items)]
    
    # Warmup Numba
    v2_engine_warm = Level2Engine(Container("warm", 100, 100, 100))
    v2_engine_warm.pack([Item("w1", 1, 1, 1)])
    
    # Level 1
    start = time.time()
    l1_engine = Level1Engine(Container("l1", 100, 100, 100))
    l1_engine.pack(items)
    l1_time = time.time() - start
    
    # Level 2
    start = time.time()
    l2_engine = Level2Engine(Container("l2", 100, 100, 100))
    l2_engine.pack(items)
    l2_time = time.time() - start
    
    print(f"Benchmark for {num_items} items:")
    print(f"Level 1 (Standard Python): {l1_time:.4f}s")
    print(f"Level 2 (Numba Accelerated): {l2_time:.4f}s")
    print(f"Speedup: {l1_time/l2_time:.2f}x")

if __name__ == "__main__":
    run_benchmark(100)
