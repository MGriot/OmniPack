import pytest
from core.models import Item, Container, LoadingStrategy
from core.engine_v2 import Level2Engine

def test_stop_id_grouping():
    container = Container("TRUCK_1", 100, 100, 100)
    
    # Discharge sequence: 1 (First), 2 (Second), 3 (Third/Last)
    item_stop1 = Item("STOP_1", 50, 50, 20, stop_id=1)
    item_stop2 = Item("STOP_2", 50, 50, 20, stop_id=2)
    item_stop3 = Item("STOP_3", 50, 50, 20, stop_id=3)
    
    engine = Level2Engine(container, stability_factor=0.5)
    engine.pack([item_stop1, item_stop2, item_stop3])
    
    p3 = next(i for i in container.items if i.id == "STOP_3").position
    assert p3[2] == 0 # Stop 3 at back
    
    acc = container.calculate_accessibility()
    print(f"\nGrouping Accessibility Score: {acc}%")
    assert acc == 100.0 # Everything is accessible at its own stop

def test_stop_id_saturation():
    container = Container("TRUCK_SMALL", 60, 60, 100)
    
    item_stop1 = Item("S_STOP_1", 50, 50, 20, stop_id=1)
    item_stop2 = Item("S_STOP_2", 50, 50, 20, stop_id=2)
    item_stop3 = Item("S_STOP_3", 50, 50, 20, stop_id=3)
    
    engine = Level2Engine(container, stability_factor=1.0)
    engine.pack([item_stop1, item_stop2, item_stop3])
    
    p1 = next(i for i in container.items if i.id == "S_STOP_1").position
    p2 = next(i for i in container.items if i.id == "S_STOP_2").position
    p3 = next(i for i in container.items if i.id == "S_STOP_3").position
    
    print(f"\nSaturated Positions - S1: {p1}, S2: {p2}, S3: {p3}")
    
    assert p3[2] == 0
    assert p2[2] == 20
    assert p1[2] == 40
    
    acc = container.calculate_accessibility()
    print(f"Saturated Accessibility Score: {acc}%")
    assert acc == 100.0 # Correct discharge order ensures 100% accessibility

def test_wrong_order_blocking():
    container = Container("TRUCK_WRONG", 60, 60, 100)
    
    # Pack Stop 1 at the BACK (wrong) and Stop 2 at the FRONT (wrong)
    # We force positions manually
    item_stop1 = Item("WRONG_1", 50, 50, 20, stop_id=1)
    item_stop1.position = (0, 0, 0) # Back
    
    item_stop2 = Item("WRONG_2", 50, 50, 20, stop_id=2)
    item_stop2.position = (0, 0, 20) # In front of WRONG_1
    
    container.items = [item_stop1, item_stop2]
    
    acc = container.calculate_accessibility()
    print(f"\nWrong Order Accessibility Score: {acc}%")
    # WRONG_1 is blocked by WRONG_2 (oz=20 >= iz=0+20).
    # Since WRONG_2 is for Stop 2, it blocks WRONG_1 (Stop 1).
    # WRONG_2 is not blocked by WRONG_1 (already gone at stop 2).
    # Accessible count = 1 (WRONG_2). 
    # Total = 2. Score = 50%
    assert acc == 50.0

if __name__ == "__main__":
    test_stop_id_grouping()
    test_stop_id_saturation()
    test_wrong_order_blocking()
