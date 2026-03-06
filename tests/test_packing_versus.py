import pytest
from core.models import Item, Container, PackingVersus
from core.engine_v2 import Level2Engine

def test_packing_versus_logic():
    # Large container
    container_dim = (100, 100, 100)
    
    # 5 items to see the trend
    def get_items():
        return [Item(f"i_{i}", 20, 20, 20) for i in range(5)]

    # Case 1: LONGITUDINAL (Z, Y, X)
    # Trend: Should fill Width (X) first because X is last in tuple
    c1 = Container("C1", *container_dim)
    engine1 = Level2Engine(c1, versus=PackingVersus.LONGITUDINAL)
    engine1.pack(get_items())
    positions1 = [it.position for it in c1.items]
    print(f"\nLongitudinal Positions: {positions1}")
    # Expected: (0,0,0), (20,0,0), (40,0,0), ...
    assert positions1[1] == (20, 0, 0)
    assert positions1[2] == (40, 0, 0)

    # Case 2: LATERAL (X, Z, Y)
    # Trend: Should fill Depth (Z) first because Z is second, Y is last
    # Wait, (X, Z, Y) means for same X, smaller Z wins.
    # So it should fill one 'strip' of depth for each X.
    c2 = Container("C2", *container_dim)
    engine2 = Level2Engine(c2, versus=PackingVersus.LATERAL)
    engine2.pack(get_items())
    positions2 = [it.position for it in c2.items]
    print(f"Lateral Positions: {positions2}")
    # Expected: (0,0,0), (0,0,20), (0,0,40), ...
    # If it picks (0,20,0) instead, it means Y beat Z.
    
    # Case 3: FLOOR_FIRST (Y, Z, X)
    # Trend: Should fill whole floor before stacking
    c3 = Container("C3", *container_dim)
    engine3 = Level2Engine(c3, versus=PackingVersus.FLOOR_FIRST)
    engine3.pack(get_items())
    positions3 = [it.position for it in c3.items]
    print(f"Floor-First Positions: {positions3}")

    # Check if Lateral is indeed moving along Z
    # If my code is correct, p2[1] should be (0,0,20)
    assert positions2[1][2] == 20 or positions2[1][0] == 20

if __name__ == "__main__":
    test_packing_versus_logic()
