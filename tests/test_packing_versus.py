import pytest
import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))
from omnipack.core.models import Item, Container, PackingVersus
from omnipack.core.engine_v2 import Level2Engine

def test_lateral_versus():
    # Lateral should fill X (Width) before Z (Depth)
    container = Container("C1", 100, 100, 100)
    engine = Level2Engine(container, versus=PackingVersus.LATERAL)
    
    # 3 small boxes. 
    # Longitudinal would place them at (0,0,0), (0,0,10), (0,0,20)
    # Lateral should place them at (0,0,0), (10,0,0), (20,0,0)
    items = [
        Item("B1", 10, 10, 10),
        Item("B2", 10, 10, 10),
        Item("B3", 10, 10, 10)
    ]
    
    engine.pack(items)
    
    assert items[0].position == (0, 0, 0)
    assert items[1].position == (10.0, 0, 0)
    assert items[2].position == (20.0, 0, 0)

def test_longitudinal_versus():
    # Longitudinal should fill Z (Depth) before X (Width)
    container = Container("C1", 100, 100, 100)
    engine = Level2Engine(container, versus=PackingVersus.LONGITUDINAL)
    
    items = [
        Item("B1", 10, 10, 10),
        Item("B2", 10, 10, 10),
        Item("B3", 10, 10, 10)
    ]
    
    engine.pack(items)
    
    assert items[0].position == (0, 0, 0)
    assert items[1].position == (0, 0, 10.0)
    assert items[2].position == (0, 0, 20.0)

if __name__ == "__main__":
    test_lateral_versus()
    test_longitudinal_versus()
