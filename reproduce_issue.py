from core.models import Item, Container
from core.engine_v2 import Level2Engine
import json

def reproduce():
    # Container: 100x100x51
    container = Container("test_bin", 100, 100, 51)
    
    # 3 items: 30x30x30, weight 10, max_top 5
    items = [
        Item(f"heavy_{i}", 30, 30, 30, weight=10, max_stack_weight=5) 
        for i in range(3)
    ]
    
    # 10 items: 20x20x20, weight 5, max_top 50
    items += [
        Item(f"light_{i}", 20, 20, 20, weight=5, max_stack_weight=50) 
        for i in range(10)
    ]
    
    engine = Level2Engine(container)
    unpacked = engine.pack(items)
    
    print(f"Total items: {len(items)}")
    print(f"Packed items: {len(container.items)}")
    print(f"Unpacked items: {len(unpacked)}")
    
    stacked_count = 0
    for it in container.items:
        if it.position[2] > 0:
            stacked_count += 1
            print(f"STACKED: {it.id} at {it.position}")
        else:
            print(f"FLOOR: {it.id} at {it.position}")
            
    if stacked_count > 0:
        print("\nISSUE CONFIRMED: items were stacked despite floor space.")
    else:
        print("\nSUCCESS: All items are on the floor.")

if __name__ == "__main__":
    reproduce()
