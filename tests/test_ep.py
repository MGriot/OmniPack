from core.models import Item, Container
from core.ep import ExtremePoint, generate_extreme_points, get_valid_ep

def test_ep_generation():
    container = Container("c1", 100, 100, 100)
    item = Item("i1", 10, 20, 30)
    item.position = (0, 0, 0)
    
    eps = generate_extreme_points(container, item)
    # Generated EP should be at (10, 0, 0), (0, 20, 0), (0, 0, 30)
    expected = [ExtremePoint(10, 0, 0), ExtremePoint(0, 20, 0), ExtremePoint(0, 0, 30)]
    
    assert len(eps) == 3
    assert all(p in expected for p in eps)

def test_get_valid_ep_boundary():
    container = Container("c1", 20, 20, 20)
    item = Item("i1", 15, 15, 15)
    
    eps = {ExtremePoint(0, 0, 0), ExtremePoint(10, 0, 0)}
    valid = get_valid_ep(container, item, eps)
    
    # (0,0,0) fits, (10,0,0) + 15 width = 25 > 20, so it fails
    assert len(valid) == 1
    assert valid[0] == ExtremePoint(0, 0, 0)

def test_get_valid_ep_collision():
    container = Container("c1", 100, 100, 100)
    # Existing item
    existing = Item("e1", 10, 10, 10)
    existing.position = (0, 0, 0)
    container.items.append(existing)
    
    # New item
    new_item = Item("n1", 10, 10, 10)
    
    eps = {ExtremePoint(0, 0, 0), ExtremePoint(10, 0, 0)}
    valid = get_valid_ep(container, new_item, eps)
    
    # (0,0,0) collides with existing, (10,0,0) is fine
    assert len(valid) == 1
    assert valid[0] == ExtremePoint(10, 0, 0)
