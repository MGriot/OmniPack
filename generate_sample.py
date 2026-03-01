import json
from core.models import Item, Container
from core.engine_v2 import Level2Engine

def generate_sample():
    container = Container("Standard_40ft", 100, 50, 50)
    items = [
        Item(f"Box_{i}", 20, 20, 20) for i in range(10)
    ] + [
        Item(f"Flat_{i}", 40, 5, 20) for i in range(5)
    ]
    
    engine = Level2Engine(container)
    engine.pack(items)
    
    with open("result.json", "w") as f:
        json.dump(container.to_dict(), f, indent=4)
    
    print("Sample result.json generated.")

if __name__ == "__main__":
    generate_sample()
