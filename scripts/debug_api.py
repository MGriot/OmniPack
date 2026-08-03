import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))

from omnipack.core.models import Item, Container
from omnipack.core.multi_container import MultiContainerEngine
import json

def test_api_logic():
    container = Container("c1", 100, 50, 50)
    items = [Item("i1", 10, 10, 10, weight=1.0, max_stack_weight=10.0)]
    
    print("Starting pack_all with mode='level2'...")
    multi_engine = MultiContainerEngine(container)
    containers = multi_engine.pack_all(items, strategy="minimize_out", mode="level2")
    
    result = {
        "containers": [c.to_dict() for c in containers],
        "total_containers": len(containers)
    }
    print("Success!")
    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    try:
        test_api_logic()
    except Exception as e:
        import traceback
        print(f"FAILED: {e}")
        traceback.print_exc()
