import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))
from omnipack.core.models import Item, Container, Rotation, ShapeType

def test_item_volume():
    item = Item("box1", 10, 20, 30)
    assert item.volume() == 6000

def test_item_rotation():
    item = Item("box1", 10, 20, 30)
    # W_H_D: (10, 20, 30)
    assert item.get_dimension() == (10, 20, 30)
    
    item.rotation = Rotation.H_W_D # (20, 10, 30)
    assert item.get_dimension() == (20, 10, 30)

def test_container_volume():
    container = Container("bin1", 100, 100, 100)
    assert container.volume() == 1_000_000

def test_container_remaining_volume():
    container = Container("bin1", 100, 100, 100)
    item = Item("box1", 10, 20, 30)
    container.items.append(item)
    assert container.remaining_volume() == 1_000_000 - 6000

def test_shape_type_propagation():
    item = Item("sphere1", 10, 10, 10, shape_type=ShapeType.SPHERE)
    assert item.shape_type == ShapeType.SPHERE
    
    container = Container("cont1", 100, 100, 100, shape_type=ShapeType.CYLINDER)
    assert container.shape_type == ShapeType.CYLINDER
