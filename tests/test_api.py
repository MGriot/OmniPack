from fastapi.testclient import TestClient
import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))
from omnipack.backend import app

client = TestClient(app)

def test_root():
    response = client.get("/api-status")
    assert response.status_code == 200
    assert response.json()["message"] == "OmniPack API is running."

def test_pack_api_level1():
    data = {
        "container": {"id": "c1", "width": 100, "height": 100, "depth": 100},
        "items": [
            {"id": "i1", "width": 50, "height": 50, "depth": 50}
        ],
        "mode": "level1"
    }
    response = client.post("/pack", json=data)
    assert response.status_code == 200
    json_data = response.json()
    assert "suggestions" in json_data
    assert len(json_data["suggestions"]) > 0
    assert "containers" in json_data["suggestions"][0]
    # Check if the item is packed in the first container of the first scenario
    assert len(json_data["suggestions"][0]["containers"][0]["items"]) == 1

def test_pack_api_genetic():
    data = {
        "container": {"id": "c1", "width": 100, "height": 100, "depth": 100},
        "items": [
            {"id": "i1", "width": 50, "height": 50, "depth": 50}
        ],
        "mode": "genetic"
    }
    response = client.post("/pack", json=data)
    assert response.status_code == 200
    json_data = response.json()
    assert "suggestions" in json_data
    # Utilization should be (50*50*50) / (100*100*100) = 125,000 / 1,000,000 = 12.5%
    # Note: genetic might not find a solution if iterations are too low, but for 1 item it should.
    assert json_data["best_utilization"] == 12.5
