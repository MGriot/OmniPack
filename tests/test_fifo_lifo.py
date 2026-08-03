import pytest
import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))
from omnipack.core.models import Item, Container, LoadingStrategy
from omnipack.core.engine_v2 import Level2Engine

def test_fifo_lifo_packing():
    container = Container("C1", 100, 100, 100)
    
    # FIFO Item (should go to back: Z=0)
    item_fifo = Item("FIFO_1", 20, 20, 20, strategy=LoadingStrategy.FIFO)
    
    # LIFO Item (should go to front: Z near 100)
    item_lifo = Item("LIFO_1", 20, 20, 20, strategy=LoadingStrategy.LIFO)
    
    # Standard Item (should go to back by default: Z=0)
    item_none = Item("NONE_1", 20, 20, 20, strategy=LoadingStrategy.NONE)
    
    engine = Level2Engine(container, stability_factor=0.5)
    unpacked = engine.pack([item_fifo, item_lifo, item_none])
    
    assert len(unpacked) == 0
    
    packed_fifo = next(i for i in container.items if i.id == "FIFO_1")
    packed_lifo = next(i for i in container.items if i.id == "LIFO_1")
    packed_none = next(i for i in container.items if i.id == "NONE_1")
    
    print(f"\nFIFO Position: {packed_fifo.position}")
    print(f"LIFO Position: {packed_lifo.position}")
    print(f"NONE Position: {packed_none.position}")
    
    # FIFO should be at back
    assert packed_fifo.position[2] == 0
    
    # LIFO should be as far front as possible
    # Given its size 20 and container depth 100, the best Z for a front-loading strategy 
    # would be 100 - 20 = 80 (if the EP exists there)
    assert packed_lifo.position[2] > 0
    assert packed_lifo.position[2] >= 50 # Roughly front half

if __name__ == "__main__":
    test_fifo_lifo_packing()
