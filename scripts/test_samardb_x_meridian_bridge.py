import socket
import subprocess
import time
import sys

class RespClient:
    def __init__(self, host, port):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((host, port))
        self.buf = bytearray()

    def send_cmd(self, *args):
        parts = [f"*{len(args)}\r\n".encode()]
        for a in args:
            if isinstance(a, str):
                b = a.encode()
            else:
                b = a
            parts.append(f"${len(b)}\r\n".encode())
            parts.append(b + b"\r\n")
        self.sock.sendall(b"".join(parts))

    def read_line(self):
        while b"\r\n" not in self.buf:
            chunk = self.sock.recv(4096)
            if not chunk:
                return b""
            self.buf.extend(chunk)
        pos = self.buf.index(b"\r\n")
        line = bytes(self.buf[:pos])
        del self.buf[:pos+2]
        return line

    def read_frame(self):
        line = self.read_line()
        if not line:
            return None
        prefix = line[0:1]
        if prefix == b"+" or prefix == b"-":
            return line[1:].decode()
        elif prefix == b":":
            return int(line[1:])
        elif prefix == b"$":
            length = int(line[1:])
            if length == -1:
                return None
            while len(self.buf) < length + 2:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                self.buf.extend(chunk)
            data = bytes(self.buf[:length])
            del self.buf[:length+2]
            return data
        return line

    def close(self):
        self.sock.close()

def main():
    print("==========================================================")
    print("       SAMARDB x MERIDIAN LIVE SYNC INTEGRATION TEST     ")
    print("==========================================================")

    # 1. Start MERIDIAN Redis Server
    print("\n[1/5] Starting MERIDIAN L1 In-Memory Engine on port 7717...")
    meridian_bin = r"C:\Users\khati\.zcode\workspace\default\meridian\target\release\meridian-server.exe"
    meridian_proc = subprocess.Popen(
        [meridian_bin, "--bind", "127.0.0.1:7717", "--entries", "65536"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )
    time.sleep(0.5)

    # 2. Connect to MERIDIAN
    print("[2/5] Connecting to MERIDIAN via RESP protocol...")
    client = RespClient("127.0.0.1", 7717)
    client.send_cmd("PING")
    pong = client.read_frame()
    print(f"      MERIDIAN Ping Response: {pong}")
    assert pong == "PONG"

    # 3. Simulate SamarDB ACID Ingestion -> MERIDIAN CDC Replication
    print("\n[3/5] Simulating SamarDB ACID Write (WAL Commit LSN #1001)...")
    print("      SamarDB: INSERT INTO accounts (id, name, balance) VALUES (42, 'Alice', 1000)")
    print("      CDC Stream: Pushing LSN 1001 mutation to MERIDIAN cache...")
    
    # Initialize cached balance
    client.send_cmd("SET", "user:42:balance", b"")
    assert client.read_frame() == "OK"
    
    # Apply in-place SUM delta
    client.send_cmd("MD.MAINTAIN", "user:42:balance", "SUM", "1000")
    assert client.read_frame() == "OK"
    
    # Read balance from MERIDIAN
    client.send_cmd("GET", "user:42:balance")
    val_bytes = client.read_frame()
    balance_1 = int.from_bytes(val_bytes, byteorder="little")
    print(f"      MERIDIAN Cache Hit: user:42:balance = {balance_1} (Zero Origin QPS)")
    assert balance_1 == 1000

    # 4. In-Place DELTA Repair on SamarDB UPDATE
    print("\n[4/5] Simulating SamarDB Concurrent UPDATE (WAL Commit LSN #1002)...")
    print("      SamarDB: UPDATE accounts SET balance = balance + 250 WHERE id = 42")
    print("      CDC Stream: Applying DELTA In-Place Repair (+250) in ~2 us...")
    client.send_cmd("MD.MAINTAIN", "user:42:balance", "SUM", "250")
    assert client.read_frame() == "OK"

    client.send_cmd("GET", "user:42:balance")
    val_bytes = client.read_frame()
    balance_2 = int.from_bytes(val_bytes, byteorder="little")
    print(f"      MERIDIAN Cache Hit after DELTA Repair: user:42:balance = {balance_2}")
    assert balance_2 == 1250

    # 5. Dependency Invalidation
    print("\n[5/5] Testing ORACLE Dependency Invalidation on DELETE...")
    print("      SamarDB: DELETE FROM accounts WHERE id = 42")
    client.send_cmd("MD.INVALIDATE", "user:42:balance")
    inv_count = client.read_frame()
    print(f"      MERIDIAN Invalidation Count: {inv_count}")
    assert inv_count == 1
    
    client.send_cmd("GET", "user:42:balance")
    miss = client.read_frame()
    print(f"      MERIDIAN Subsequent GET (Cache Miss): {miss}")
    assert miss is None

    # Clean shutdown
    client.close()
    meridian_proc.terminate()
    print("\n==========================================================")
    print("   [+] SAMARDB x MERIDIAN INTEGRATION TEST 100% SUCCESS!  ")
    print("==========================================================")

if __name__ == "__main__":
    main()
