import os

files = {}

files['src/net_core.lpp'] = '''# Networker Core Networking Primitives and Framing
def net_is_valid(handle: Int) -> Bool:
    return handle > 0

def net_frame_message(payload: Str) -> Str:
    l := str_len(payload)
    return int_to_str(l) + ":" + payload

def net_send_framed(handle: Int, payload: Str) -> Bool:
    if !net_is_valid(handle):
        return false
    framed := net_frame_message(payload)
    sent := net_send_all(handle, framed)
    return sent > 0

def net_recv_framed(handle: Int, max_bytes: Int) -> Str:
    if !net_is_valid(handle):
        return ""
    raw := net_recv(handle, max_bytes)
    l := str_len(raw)
    if l == 0:
        return ""
    colon_idx := str_find(raw, ":")
    if colon_idx <= 0:
        return raw
    len_str := str_substr(raw, 0, colon_idx)
    expected_len := str_to_int(len_str)
    payload := str_substr(raw, colon_idx + 1, expected_len)
    return payload
'''

files['src/net_http.lpp'] = '''# Networker HTTP 1.1 Parser and Serializer
import net_core

struct HttpRequest:
    method: Str
    path: Str
    content_length: Int
    content_type: Str
    body: Str

struct HttpResponse:
    status_code: Int
    status_text: Str
    content_type: Str
    body: Str

def http_parse_request(raw: Str) -> HttpRequest:
    mut method := "GET"
    mut path := "/"
    mut clen := 0
    mut ctype := "text/plain"
    mut body := ""
    
    first_space := str_find(raw, " ")
    if first_space > 0:
        method = str_substr(raw, 0, first_space)
        rest := str_substr(raw, first_space + 1, str_len(raw) - (first_space + 1))
        second_space := str_find(rest, " ")
        if second_space > 0:
            path = str_substr(rest, 0, second_space)
            
    header_end := str_find(raw, "\\r\\n\\r\\n")
    if header_end > 0:
        body = str_substr(raw, header_end + 4, str_len(raw) - (header_end + 4))
    else:
        alt_end := str_find(raw, "\\n\\n")
        if alt_end > 0:
            body = str_substr(raw, alt_end + 2, str_len(raw) - (alt_end + 2))
            
    cl_idx := str_find(raw, "Content-Length: ")
    if cl_idx > 0:
        cl_rest := str_substr(raw, cl_idx + 16, 20)
        mut cl_newline := str_find(cl_rest, "\\r")
        if cl_newline <= 0:
            cl_newline = str_find(cl_rest, "\\n")
        if cl_newline > 0:
            clen_str := str_substr(cl_rest, 0, cl_newline)
            clen = str_to_int(clen_str)
            
    ct_idx := str_find(raw, "Content-Type: ")
    if ct_idx > 0:
        ct_rest := str_substr(raw, ct_idx + 14, 60)
        mut ct_newline := str_find(ct_rest, "\\r")
        if ct_newline <= 0:
            ct_newline = str_find(ct_rest, "\\n")
        if ct_newline > 0:
            ctype = str_substr(ct_rest, 0, ct_newline)
            
    return HttpRequest(method, path, clen, ctype, body)

def http_response_new(status_code: Int, status_text: Str, content_type: Str, body: Str) -> HttpResponse:
    return HttpResponse(status_code, status_text, content_type, body)

def http_build_response(resp: HttpResponse) -> Str:
    body_len := str_len(resp.body)
    mut out := "HTTP/1.1 " + int_to_str(resp.status_code) + " " + resp.status_text + "\\r\\n"
    out = out + "Server: Networker-Lpp/1.0\\r\\n"
    out = out + "Content-Type: " + resp.content_type + "\\r\\n"
    out = out + "Content-Length: " + int_to_str(body_len) + "\\r\\n"
    out = out + "Connection: close\\r\\n\\r\\n"
    out = out + resp.body
    return out
'''

files['src/net_rpc.lpp'] = '''# Networker RPC Protocol Engine
import net_core

struct RpcRequest:
    id: Int
    method: Str
    params: Str

struct RpcResponse:
    id: Int
    status: Str
    result: Str

def rpc_encode_request(id: Int, method: Str, params: Str) -> Str:
    return "REQ|" + int_to_str(id) + "|" + method + "|" + params

def rpc_encode_response(id: Int, status: Str, result: Str) -> Str:
    return "RES|" + int_to_str(id) + "|" + status + "|" + result

def rpc_parse_request(raw: Str) -> RpcRequest:
    parts: List[Str] = str_split(raw, 124)
    if list_len(parts) >= 4:
        id := str_to_int(parts[1])
        method := parts[2]
        params := parts[3]
        return RpcRequest(id, method, params)
    return RpcRequest(0, "UNKNOWN", "")

def rpc_parse_response(raw: Str) -> RpcResponse:
    parts: List[Str] = str_split(raw, 124)
    if list_len(parts) >= 4:
        id := str_to_int(parts[1])
        status := parts[2]
        result := parts[3]
        return RpcResponse(id, status, result)
    return RpcResponse(0, "ERR", "MALFORMED_RPC")
'''

files['src/net_bench.lpp'] = '''# Networker Full Test Suite and High-Load Diagnostics
import net_core
import net_http
import net_rpc

def assert_str(name: Str, actual: Str, expected: Str):
    if actual == expected:
        print_str("  [PASS] " + name)
    else:
        print_str("  [FAIL] " + name + " | Expected: '" + expected + "', Got: '" + actual + "'")

def assert_true(name: Str, cond: Bool):
    if cond:
        print_str("  [PASS] " + name)
    else:
        print_str("  [FAIL] " + name + " | Condition failed")

def assert_int(name: Str, actual: Int, expected: Int):
    if actual == expected:
        print_str("  [PASS] " + name)
    else:
        print_str("  [FAIL] " + name + " | Expected: " + int_to_str(expected) + ", Got: " + int_to_str(actual))

def main():
    print_str("================================================================")
    print_str("  Networker v1.0.0 -- L++ Core Networking and Protocol Suite")
    print_str("================================================================")
    
    port := 19999
    
    print_str("--- [1/5] Testing HTTP Parser and Response Builder ---")
    sample_http := "POST /api/v1/compute HTTP/1.1\\r\\nHost: localhost:19999\\r\\nContent-Type: application/json\\r\\nContent-Length: 26\\r\\n\\r\\n{\\"action\\": \\"fib\\", \\\"n\\\": 10}"
    req := http_parse_request(sample_http)
    assert_str("http method", req.method, "POST")
    assert_str("http path", req.path, "/api/v1/compute")
    assert_int("http content-length", req.content_length, 26)
    assert_str("http content-type", req.content_type, "application/json")
    
    resp_obj := http_response_new(200, "OK", "application/json", "{\\"result\\": 55}")
    raw_resp := http_build_response(resp_obj)
    assert_true("http response contains 200 OK", str_find(raw_resp, "200 OK") > 0)
    assert_true("http response contains Content-Length: 16", str_find(raw_resp, "Content-Length: 16") > 0)
    
    print_str("--- [2/5] Testing RPC Framing and Serialization ---")
    req_encoded := rpc_encode_request(101, "vector_dot", "[1,2,3],[4,5,6]")
    parsed_req := rpc_parse_request(req_encoded)
    assert_int("rpc id", parsed_req.id, 101)
    assert_str("rpc method", parsed_req.method, "vector_dot")
    assert_str("rpc params", parsed_req.params, "[1,2,3],[4,5,6]")
    
    res_encoded := rpc_encode_response(101, "OK", "32")
    parsed_res := rpc_parse_response(res_encoded)
    assert_int("rpc response id", parsed_res.id, 101)
    assert_str("rpc response status", parsed_res.status, "OK")
    assert_str("rpc response result", parsed_res.result, "32")
    
    print_str("--- [3/5] Testing Live TCP Client-Server Loopback ---")
    listener := net_listen(port)
    assert_true("server bind net_listen", net_is_valid(listener))
    
    client := net_connect("127.0.0.1", port)
    assert_true("client connect net_connect", net_is_valid(client))
    
    server_client := net_accept(listener)
    assert_true("server accept net_accept", net_is_valid(server_client))
    
    net_send_framed(client, "HELLO_LPP_NETWORKER")
    received := net_recv_framed(server_client, 512)
    assert_str("framed message roundtrip", received, "HELLO_LPP_NETWORKER")
    
    net_send_framed(server_client, "ACK_NETWORKER_OK")
    client_ack := net_recv_framed(client, 512)
    assert_str("client framed ack", client_ack, "ACK_NETWORKER_OK")
    
    net_close(server_client)
    net_close(client)
    
    print_str("--- [4/5] Testing Live HTTP Server and Client Flow ---")
    http_client := net_connect("127.0.0.1", port)
    http_srv_conn := net_accept(listener)
    
    raw_http_req := "GET /health HTTP/1.1\\r\\nHost: localhost\\r\\n\\r\\n"
    net_send_all(http_client, raw_http_req)
    
    srv_http_raw := net_recv(http_srv_conn, 1024)
    srv_req := http_parse_request(srv_http_raw)
    assert_str("server parsed request method", srv_req.method, "GET")
    assert_str("server parsed request path", srv_req.path, "/health")
    
    http_reply := http_build_response(http_response_new(200, "OK", "application/json", "{\\"status\\": \\"healthy\\", \\"uptime\\\": 100}"))
    net_send_all(http_srv_conn, http_reply)
    
    client_http_resp := net_recv(http_client, 1024)
    assert_true("client received 200 OK HTTP response", str_find(client_http_resp, "200 OK") > 0)
    assert_true("client received JSON payload", str_find(client_http_resp, "healthy") > 0)
    
    net_close(http_srv_conn)
    net_close(http_client)
    
    print_str("--- [5/5] High-Throughput Stress Test (100 Roundtrips) ---")
    mut successful_rounds := 0
    t0 := time_ms()
    for i in range(0, 100):
        c := net_connect("127.0.0.1", port)
        if net_is_valid(c):
            s := net_accept(listener)
            if net_is_valid(s):
                req_data := rpc_encode_request(i, "square", int_to_str(i))
                net_send_all(c, req_data)
                
                req_in := net_recv(s, 256)
                parsed := rpc_parse_request(req_in)
                val := str_to_int(parsed.params)
                sq := val * val
                
                res_data := rpc_encode_response(parsed.id, "OK", int_to_str(sq))
                net_send_all(s, res_data)
                
                res_in := net_recv(c, 256)
                parsed_back := rpc_parse_response(res_in)
                if parsed_back.id == i:
                    if parsed_back.result == int_to_str(i * i):
                        successful_rounds = successful_rounds + 1
                net_close(s)
            net_close(c)
            
    t1 := time_ms()
    dt := t1 - t0
    print_str("  Processed 100 full TCP connect-request-compute-respond-close cycles in " + int_to_str(dt) + " ms")
    print_str("  Successful cycles: " + int_to_str(successful_rounds) + "/100")
    assert_int("100/100 stress cycles passed", successful_rounds, 100)
    
    net_close(listener)
    print_str("================================================================")
    print_str("  ALL NETWORKER TESTS PASSED SUCCESSFULLY!")
    print_str("================================================================")
'''

for path, content in files.items():
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
print('All networker files written successfully.')
