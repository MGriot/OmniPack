from fastapi.testclient import TestClient
from main import app

client = TestClient(app)

def test_root():
    response = client.get("/")
    assert response.status_code == 200
    assert response.json() == {"message": "OmniPack API is running. Use /pack to optimize space."}

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
    assert "container" in json_data
    assert len(json_data["container"]["items"]) == 1

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
    assert "container" in json_data
    # Utilization should be (50*50*50) / (100*100*100) = 125,000 / 1,000,000 = 12.5%
    assert json_data["container"]["utilization"] == 12.5
