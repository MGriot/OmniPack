import numpy as np
from core.models import Item, Container
from core.engine_v2 import Level2Engine

def test_grasp_diversity():
    # Identical setup for two runs
    def run_engine(k):
        container = Container("C", 100, 100, 100)
        # Multiple items that could fit in many spots
        items = [Item(f"I_{i}", 30, 30, 30) for i in range(5)]
        engine = Level2Engine(container, stability_factor=0.5)
        engine.pack(items, grasp_k=k)
        return [tuple(it.position) for it in container.items]

    # With k=1, results should be deterministic
    pos1_k1 = run_engine(1)
    pos2_k1 = run_engine(1)
    print(f"\nk=1 Run 1: {pos1_k1}")
    print(f"k=1 Run 2: {pos2_k1}")
    assert pos1_k1 == pos2_k1

    # With k=10, results should (likely) be different across runs
    # We try a few times because random chance might pick the same top spot
    different = False
    first_run_k10 = run_engine(10)
    print(f"k=10 Run 1: {first_run_k10}")
    
    for i in range(10):
        next_run = run_engine(10)
        if next_run != first_run_k10:
            print(f"k=10 Run {i+2}: {next_run} (DIFFERENT!)")
            different = True
            break
    
    assert different, "GRASP k=10 failed to produce a different layout in 10 attempts"

if __name__ == "__main__":
    test_grasp_diversity()
