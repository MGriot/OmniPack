import time
import random
import numpy as np
from omnipack_core import RTreeManager, Rect3D
from core.accelerated import get_overlap_area

def numba_collision_check(new_pos, new_dim, existing_pos, existing_dim):
    nx, ny, nz = new_pos
    nw, nh, nd = new_dim
    for k in range(len(existing_pos)):
        ox, oy, oz = existing_pos[k]
        ow, oh, od = existing_dim[k]
        if (nx < ox + ow - 0.001 and nx + nw > ox + 0.001 and
            ny < oy + oh - 0.001 and ny + nh > oy + 0.001 and
            nz < oz + od - 0.001 and nz + nd > oz + 0.001):
            return True
    return False

def test_rtree_correctness():
    manager = RTreeManager()
    
    # Add some items
    manager.insert(Rect3D(0, 0, 0, 10, 10, 10))
    manager.insert(Rect3D(20, 20, 20, 30, 30, 30))
    
    # Test intersection
    assert manager.intersects(Rect3D(5, 5, 5, 15, 15, 15)) == True  # Overlaps first
    assert manager.intersects(Rect3D(15, 15, 15, 18, 18, 18)) == False # Gap
    assert manager.intersects(Rect3D(25, 25, 25, 35, 35, 35)) == True  # Overlaps second
    print("\nCorrectness check passed!")

def benchmark_collision():
    num_existing = 1000
    num_checks = 100
    
    # Generate random existing items
    existing_pos = np.random.rand(num_existing, 3) * 100
    existing_dim = np.random.rand(num_existing, 3) * 10
    
    # Setup Rust R-Tree
    manager = RTreeManager()
    for i in range(num_existing):
        p = existing_pos[i]
        d = existing_dim[i]
        manager.insert(Rect3D(p[0], p[1], p[2], p[0]+d[0], p[1]+d[1], p[2]+d[2]))
        
    # Generate random checks
    checks_pos = np.random.rand(num_checks, 3) * 100
    checks_dim = np.random.rand(num_checks, 3) * 10
    
    # Bench Numba (Linear search O(N))
    start = time.perf_counter()
    numba_hits = 0
    for i in range(num_checks):
        if numba_collision_check(checks_pos[i], checks_dim[i], existing_pos, existing_dim):
            numba_hits += 1
    numba_time = time.perf_counter() - start
    
    # Bench Rust R-Tree (Logarithmic search O(log N))
    start = time.perf_counter()
    rust_hits = 0
    for i in range(num_checks):
        rect = Rect3D(checks_pos[i,0], checks_pos[i,1], checks_pos[i,2], 
                      checks_pos[i,0]+checks_dim[i,0], checks_pos[i,1]+checks_dim[i,1], checks_pos[i,2]+checks_dim[i,2])
        if manager.intersects(rect):
            rust_hits += 1
    rust_time = time.perf_counter() - start
    
    print(f"\nBenchmark Results ({num_existing} items, {num_checks} checks):")
    print(f"Numba Linear Time: {numba_time:.6f}s (Hits: {numba_hits})")
    print(f"Rust R-Tree Time:  {rust_time:.6f}s (Hits: {rust_hits})")
    print(f"Speedup: {numba_time/rust_time:.2f}x")
    
    assert numba_hits == rust_hits, f"Hit mismatch! Numba: {numba_hits}, Rust: {rust_hits}"

if __name__ == "__main__":
    test_rtree_correctness()
    benchmark_collision()
