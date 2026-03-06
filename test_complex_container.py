import pytest
import numpy as np
from core.models import Item, Container, ShapePart, Rotation
from core.engine_v2 import Level2Engine

def test_l_shaped_container():
    # Define an L-shaped container
    # Part 1: Bottom (100x50x50)
    # Part 2: Vertical (50x50x50) on top of the left side
    container_parts = [
        ShapePart(0, 0, 0, 100, 50, 50),
        ShapePart(0, 50, 0, 50, 50, 50)
    ]
    container = Container("LC1", 100, 100, 50, parts=container_parts)
    
    engine = Level2Engine(container)
    
    # 1. This item fits in the bottom part
    item1 = Item("I1", 30, 30, 30)
    unpacked = engine.pack([item1])
    assert len(unpacked) == 0
    assert container.items[0].position == (0, 0, 0)
    
    # 2. This item should fit in the vertical extension (X=0, Y=50, Z=0)
    item2 = Item("I2", 40, 40, 40)
    # If it's placed on floor, it should prefer (30, 0, 0) if it fits there
    # But let's see where it goes.
    unpacked = engine.pack([item2])
    assert len(unpacked) == 0
    
    # 4. This item fits bounding box (100x100x50) but is OUTSIDE the parts union
    # Union is: (0,0,0)-(100,50,50) OR (0,50,0)-(50,100,50)
    # Let's try placing a 10x10x10 item at (60, 60, 0)
    # The engine should not find any valid EP for it if we don't have parts there.
    # We'll use get_valid_ep to check.
    from core.ep import get_valid_ep
    
    # Create a dummy EP at (60, 60, 0)
    from core.ep import ExtremePoint
    dummy_ep = ExtremePoint(60, 60, 0)
    
    test_item = Item("Test", 10, 10, 10)
    valid_eps = get_valid_ep(container, test_item, {dummy_ep})
    
    assert len(valid_eps) == 0, "Item at (60, 60, 0) should be invalid as it is outside container parts"
    
    print("Complex container boundary check passed.")

if __name__ == "__main__":
    test_l_shaped_container()
