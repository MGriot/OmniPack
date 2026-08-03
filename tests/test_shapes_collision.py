import pytest
import numpy as np
import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))
from omnipack.core.models import Item, Container, ShapeType
from omnipack.core.engine_v2 import Level2Engine

def test_sphere_collision():
    # Sphere 1 at (0,0,0) diameter 10. Center (5,5,5)
    # Sphere 2 at (5,0,0) diameter 10. Center (10,5,5)
    # Dist = 5. Radii sum = 10. 5 < 10 -> COLLIDE
    container = Container("C1", 100, 100, 100)
    
    s1 = Item("S1", 10, 10, 10, shape_type=ShapeType.SPHERE)
    s1.position = (0, 0, 0)
    container.items.append(s1)
    
    s2 = Item("S2", 10, 10, 10, shape_type=ShapeType.SPHERE)
    from omnipack.core.ep import ExtremePoint, get_valid_ep
    
    valid = get_valid_ep(container, s2, {ExtremePoint(5, 0, 0)})
    assert len(valid) == 0, "Overlapping spheres should collide"
    
    valid = get_valid_ep(container, s2, {ExtremePoint(10, 0, 0)})
    assert len(valid) == 1, "Touching spheres should not collide"

def test_sphere_box_collision():
    container = Container("C1", 100, 100, 100)
    
    # Box at (0,0,0) size (10,10,10)
    b1 = Item("B1", 10, 10, 10, shape_type=ShapeType.BOX)
    b1.position = (0, 0, 0)
    container.items.append(b1)
    
    s1 = Item("S1", 10, 10, 10, shape_type=ShapeType.SPHERE)
    from omnipack.core.ep import ExtremePoint, get_valid_ep
    
    # Sphere at (8, 0, 0) -> Center (13, 5, 5). Closest point on box is (10, 5, 5). Dist 3 < 5 -> COLLIDE
    valid = get_valid_ep(container, s1, {ExtremePoint(8, 0, 0)})
    assert len(valid) == 0, "Sphere at (8,0,0) should collide with box at (0,0,0)"
    
    # Sphere at (15, 0, 0) -> Center (20, 5, 5). Dist 10 > 5 -> NO COLLIDE
    valid = get_valid_ep(container, s1, {ExtremePoint(15, 0, 0)})
    assert len(valid) == 1, "Sphere at (15,0,0) should not collide with box at (0,0,0)"

def test_level2_accelerated_path():
    # Ensure Level2Engine uses the new collision logic
    container = Container("C1", 100, 100, 100)
    engine = Level2Engine(container)
    
    # Box at (0,0,0)
    b1 = Item("B1", 10, 10, 10, shape_type=ShapeType.BOX)
    b1.position = (0, 0, 0)
    container.items.append(b1)
    
    # Try to pack a sphere.
    # Standard Bounding Box would force it to at least (10, 0, 0)
    # But a sphere center (14, 5, 5) diameter 10 has radius 5.
    # Closest point on box is (10, 5, 5). Dist 4 < 5 -> COLLIDE.
    # Sphere at (9, 0, 0) -> Center (14, 5, 5).
    
    s1 = Item("S1", 10, 10, 10, shape_type=ShapeType.SPHERE)
    
    # Add EP at (9, 0, 0)
    from omnipack.core.ep import ExtremePoint
    engine.extreme_points.add(ExtremePoint(9, 0, 0))
    engine.extreme_points.add(ExtremePoint(15, 0, 0))
    
    unpacked = engine.pack([s1])
    
    assert len(unpacked) == 0
    # It should NOT be at (9,0,0)
    assert s1.position != (9.0, 0.0, 0.0), "Should have avoided collision at (9,0,0)"
    # It should be at (15,0,0) or (10,0,0) (10,0,0 is safe for sphere vs box at 0,0,0)
    assert s1.position[0] >= 10.0
