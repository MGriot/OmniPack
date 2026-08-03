from fastapi.testclient import TestClient
import os
import json
import sys
# Ensure src is in path
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src"))

from omnipack.backend import app
from omnipack.logic import CATALOG_PATH

client = TestClient(app)

def test_catalog_lifecycle():
    # 1. Clear existing catalog if any
    if os.path.exists(CATALOG_PATH):
        os.remove(CATALOG_PATH)
    
    # 2. Get empty catalog
    response = client.get("/catalog")
    assert response.status_code == 200
    assert response.json() == {"items": []}
    
    # 3. Save entry
    entry = {
        "name": "Test Configuration",
        "container": {"id": "c1", "width": 100, "height": 100, "depth": 100},
        "items": [
            {"id": "i1", "width": 50, "height": 50, "depth": 50}
        ]
    }
    response = client.post("/catalog", json=entry)
    assert response.status_code == 200
    assert response.json()["name"] == "Test Configuration"
    
    # 4. Verify catalog content
    response = client.get("/catalog")
    data = response.json()
    assert len(data["items"]) == 1
    assert data["items"][0]["name"] == "Test Configuration"
    
    # 5. Update same entry
    entry["container"]["width"] = 200
    client.post("/catalog", json=entry)
    response = client.get("/catalog")
    data = response.json()
    assert len(data["items"]) == 1
    assert data["items"][0]["container"]["width"] == 200

    # Cleanup
    if os.path.exists(CATALOG_PATH):
        os.remove(CATALOG_PATH)
