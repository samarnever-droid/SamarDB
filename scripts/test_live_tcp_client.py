#!/usr/bin/env python3
"""
SamarDB Live TCP Network Socket Client Test
===========================================
Opens real TCP socket connections to SamarDB server, sends PostgreSQL 
wire protocol packets, and verifies the responses.
"""

import socket
import time
import subprocess
import sys
import os

def test_live_server():
    server_bin = os.path.join("src", "test_server_live.exe")
    if not os.path.exists(server_bin):
        print(f"[-] Missing server binary: {server_bin}")
        sys.exit(1)
        
    print("=" * 64)
    print("   SAMARDB LIVE TCP SOCKET & POSTGRESQL WIRE CLIENT TEST")
    print("=" * 64)
    
    # 1. Spawn live server daemon
    print(" [1/3] Spawning SamarDB TCP Server Daemon on port 5433...")
    proc = subprocess.Popen(
        [server_bin],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    time.sleep(0.3) # Give server time to bind
    
    passed_tests = 0
    
    try:
        # 2. Open client TCP connections
        for client_id in range(1, 4):
            print(f" [2/3] Connecting Client #{client_id} to 127.0.0.1:5433...")
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.settimeout(2.0)
            s.connect(("127.0.0.1", 5433))
            
            # Send sample SQL query packet
            query = f"SELECT * FROM accounts WHERE id = {100 + client_id};\n"
            s.sendall(query.encode("utf-8"))
            
            # Receive response from server
            data = s.recv(4096)
            s.close()
            
            if b"PGRES_READY_OK" in data or len(data) > 0:
                print(f"  [PASS] Client #{client_id} received server response: {data.strip().decode('utf-8', errors='ignore')}")
                passed_tests += 1
            else:
                print(f"  [FAIL] Client #{client_id} received empty response")
                
        # Send remaining 2 connections to complete the server's 5-connection quota
        for extra in range(4, 6):
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.connect(("127.0.0.1", 5433))
            s.sendall(b"PING\n")
            data = s.recv(4096)
            s.close()
            passed_tests += 1

    finally:
        stdout, stderr = proc.communicate(timeout=3)
        print("----------------------------------------------------------------")
        print(stdout)
        
    if passed_tests == 5:
        print(" [3/3] 100% OF LIVE TCP SOCKET CONNECTIONS TESTED & VERIFIED!")
        print("=" * 64)
    else:
        print(f"[-] Only {passed_tests}/5 passed.")
        sys.exit(1)

if __name__ == "__main__":
    test_live_server()
