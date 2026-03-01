from core.models import Item, Container, Rotation
from core.engine import Level1Engine

def test_simple_pack():
    container = Container("c1", 100, 100, 100)
    items = [
        Item("i1", 50, 50, 50),
        Item("i2", 50, 50, 50)
    ]
    
    engine = Level1Engine(container)
    unpacked = engine.pack(items)
    
    assert len(unpacked) == 0
    assert len(container.items) == 2
    # First item should be at (0,0,0)
    assert container.items[0].position == (0.0, 0.0, 0.0)
    # Second item should be at one of the EPs of the first item
    # e.g., (50, 0, 0) due to our scoring (Z, Y, X)
    assert container.items[1].position in [(50.0, 0.0, 0.0), (0.0, 50.0, 0.0), (0.0, 0.0, 50.0)]

def test_rotation_fit():
    # Container is 10x50x10
    container = Container("c1", 10, 50, 10)
    # Item is 50x10x10 - Must rotate to fit
    item = Item("i1", 50, 10, 10)
    
    engine = Level1Engine(container)
    unpacked = engine.pack([item])
    
    assert len(unpacked) == 0
    assert len(container.items) == 1
    # Check if dimension matches container
    dim = container.items[0].get_dimension()
    assert dim[0] <= container.width
    assert dim[1] <= container.height
    assert dim[2] <= container.depth

def test_overflow():
    container = Container("c1", 10, 10, 10)
    items = [
        Item("i1", 10, 10, 10),
        Item("i2", 10, 10, 10)
    ]
    
    engine = Level1Engine(container)
    unpacked = engine.pack(items)
    
    assert len(unpacked) == 1
    assert len(container.items) == 1
