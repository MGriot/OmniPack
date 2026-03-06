import pytest
from core.models import Item, Container, Rotation
from core.engine_v2 import Level2Engine

def test_restricted_rotations():
    # Container is SHORT (height 20)
    container = Container("SHORT_BIN", 100, 20, 100)
    
    # Item is TALL (height 50) if not rotated
    # Original (W, H, D) = (10, 50, 10)
    
    # Case 1: All rotations allowed (should succeed by rotating to H_W_D or similar)
    item_free = Item("FREE_1", 10, 50, 10)
    engine1 = Level2Engine(container, stability_factor=0.5)
    unpacked1 = engine1.pack([item_free])
    
    assert len(unpacked1) == 0
    assert container.items[0].rotation != Rotation.W_H_D
    print(f"\nFree item rotated to: {container.items[0].rotation.name}")

    # Case 2: Only W_H_D allowed (should fail to pack because 50 > 20)
    container2 = Container("SHORT_BIN_2", 100, 20, 100)
    item_restricted = Item("RESTRICTED_1", 10, 50, 10, allowed_rotations=[Rotation.W_H_D])
    engine2 = Level2Engine(container2, stability_factor=0.5)
    unpacked2 = engine2.pack([item_restricted])
    
    assert len(unpacked2) == 1
    assert len(container2.items) == 0
    print("Restricted item correctly failed to pack.")

    # Case 3: Only specific valid rotation allowed
    container3 = Container("SHORT_BIN_3", 100, 20, 100)
    item_specific = Item("SPECIFIC_1", 10, 50, 10, allowed_rotations=[Rotation.H_W_D])
    engine3 = Level2Engine(container3, stability_factor=0.5)
    unpacked3 = engine3.pack([item_specific])
    
    assert len(unpacked3) == 0
    assert container3.items[0].rotation == Rotation.H_W_D
    print("Item correctly packed with specific allowed rotation.")

if __name__ == "__main__":
    test_restricted_rotations()
