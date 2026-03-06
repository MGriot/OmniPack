import pytest
import numpy as np
from core.models import Item, Container, ShapePart, Rotation
from core.engine_v2 import Level2Engine
from core.ep import generate_extreme_points, get_valid_ep

def test_l_shape_void_usability():
    container = Container("C1", 100, 100, 100)
    
    # L-shape: Base (20x10x10), Vertical (10x10x10) at (0,10,0)
    parts = [
        ShapePart(0, 0, 0, 20, 10, 10),
        ShapePart(0, 10, 0, 10, 10, 10)
    ]
    item = Item("L1", 20, 20, 10, parts=parts)
    item.position = (0, 0, 0)
    container.items.append(item)
    
    # Generate EPs
    eps = set(generate_extreme_points(container, item))
    ep_tuples = [e.to_tuple() for e in eps]
    
    print(f"Generated EPs: {ep_tuples}")
    
    # (10, 10, 0) should be there
    assert (10.0, 10.0, 0.0) in ep_tuples
    
    # Check if a 10x10x10 item is valid at (10, 10, 0)
    void_item = Item("V1", 10, 10, 10)
    valid_eps = get_valid_ep(container, void_item, eps)
    valid_ep_tuples = [e.to_tuple() for e in valid_eps]
    
    print(f"Valid EPs for 10x10x10: {valid_ep_tuples}")
    assert (10.0, 10.0, 0.0) in valid_ep_tuples

def test_collision_detection_compound():
    container = Container("C1", 100, 100, 100)
    parts = [
        ShapePart(0, 0, 0, 20, 10, 10),
        ShapePart(0, 10, 0, 10, 10, 10)
    ]
    item = Item("L1", 20, 20, 10, parts=parts)
    item.position = (0, 0, 0)
    container.items.append(item)
    
    # This item SHOULD collide with the vertical part of L1
    # Vertical part is (0,10,0) to (10,20,10)
    # Colliding item: (5, 15, 0) size 10x10x10 -> (5,15,0) to (15,25,10)
    colliding_item = Item("C1", 10, 10, 10)
    colliding_item.position = (5, 15, 0)
    
    # Use get_valid_ep logic (internal check)
    from core.ep import get_valid_ep
    valid = get_valid_ep(container, colliding_item, {type('EP', (), {'x': 5, 'y': 15, 'z': 0, 'to_tuple': lambda self: (self.x, self.y, self.z)})()})
    
    assert len(valid) == 0, "Should have collided with the vertical part of the L-shape"

if __name__ == "__main__":
    test_l_shape_void_usability()
    test_collision_detection_compound()
