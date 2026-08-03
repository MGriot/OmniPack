import os
import subprocess
import sys

def cleanup_port(port=8000):
    print(f"Cleaning up port {port}...")
    if os.name == 'nt':  # Windows
        try:
            output = subprocess.check_output(f"netstat -ano | findstr :{port}", shell=True).decode()
            for line in output.strip().split('\n'):
                if 'LISTENING' in line:
                    pid = line.strip().split()[-1]
                    print(f"Killing process {pid} on port {port}...")
                    subprocess.run(f"taskkill /F /PID {pid}", shell=True)
        except subprocess.CalledProcessError:
            print(f"No process found on port {port}.")
    else:  # Linux/Mac
        try:
            subprocess.run(f"fuser -k {port}/tcp", shell=True)
        except Exception as e:
            print(f"Error cleaning up port: {e}")

if __name__ == "__main__":
    cleanup_port()
    print("Cleanup complete.")
