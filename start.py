import os
import subprocess
import sys
import time
# Add scripts dir to path to allow import if running from root
sys.path.append(os.path.join(os.path.dirname(__file__), "scripts"))
from cleanup import cleanup_port

def start_server():
    cleanup_port(8000)
    print("Starting OmniPack API on http://127.0.0.1:8000 ...")
    
    # Run uvicorn via uv
    cmd = ["uv", "run", "uvicorn", "main:app", "--host", "127.0.0.1", "--port", "8000"]
    
    try:
        subprocess.run(cmd)
    except KeyboardInterrupt:
        print("\nStopping server...")
    except Exception as e:
        print(f"Error starting server: {e}")

if __name__ == "__main__":
    start_server()
