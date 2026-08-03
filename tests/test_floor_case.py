import httpx
import json

def run_test():
    url = "http://127.0.0.1:8000/pack"
    payload = {
        "container": {"id": "test", "width": 100, "height": 100, "depth": 51},
        "items": [
            {"id": f"heavy_{i}", "width": 30, "height": 30, "depth": 30, "weight": 10, "max_stack_weight": 5} 
            for i in range(3)
        ] + [
            {"id": f"light_{i}", "width": 20, "height": 20, "depth": 20, "weight": 5, "max_stack_weight": 50} 
            for i in range(10)
        ],
        "mode": "level2",
        "strategy": "minimize_out",
        "iterations": 1
    }
    
    print("Sending request to API...")
    r = httpx.post(url, json=payload, timeout=30.0)
    if r.status_code != 200:
        print(f"API ERROR: {r.text}")
        return

    res = r.json()
    sug = res['suggestions'][0]
    items = sug['containers'][0]['items']
    
    floor_items = [it['id'] for it in items if it['position']['z'] == 0]
    stacked_items = [it['id'] for it in items if it['position']['z'] > 0]
    
    print(f"Total Items Packed: {len(items)}")
    print(f"Floor Items: {len(floor_items)}")
    print(f"Stacked Items: {len(stacked_items)}")
    
    if len(stacked_items) > 0:
        print("RESULT: Stacking detected (FAIL if floor space was available)")
    else:
        print("RESULT: All items are on the floor (SUCCESS)")

if __name__ == "__main__":
    run_test()
