#!/usr/bin/env python3
"""
Integration Test for DataFold HTTP Server

This test verifies the complete workflow:
1. Starts the HTTP server
2. Loads schemas from available_schemas directory
3. Verifies all schemas are discovered and accessible
4. Approves the BlogPost schema
5. Creates a mutation to write a blog post
6. Queries the schema to verify the data
7. Cleans up by stopping the server

All operations are performed using curl commands against the HTTP API.

Usage:
    python3 tests/integration_test_http.py

The test will:
    - Automatically start and stop the HTTP server
    - Create a test blog post with timestamp-based data
    - Validate the complete create -> query workflow
    - Exit with code 0 on success, 1 on failure

Output:
    - Detailed progress for each test step
    - ✅ PASS for successful tests
    - ❌ FAIL with error details for failed tests
    - Final summary with pass/fail counts

Requirements:
    - Python 3.6+
    - Rust and Cargo (for building the server)
    - curl command available
    - BlogPost schema in available_schemas/
"""

import subprocess
import json
import time
import sys
import signal
import os
from datetime import datetime

# Configuration
HTTP_PORT = 9001
BASE_URL = f"http://localhost:{HTTP_PORT}"
SERVER_START_TIMEOUT = 30  # seconds
SERVER_PID = None


class TestResult:
    """Track test results"""
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []
    
    def add_pass(self, test_name):
        self.passed += 1
        print(f"✅ PASS: {test_name}")
    
    def add_fail(self, test_name, reason):
        self.failed += 1
        error_msg = f"❌ FAIL: {test_name} - {reason}"
        self.errors.append(error_msg)
        print(error_msg)
    
    def summary(self):
        total = self.passed + self.failed
        print("\n" + "=" * 80)
        print("TEST SUMMARY")
        print("=" * 80)
        print(f"Total tests: {total}")
        print(f"Passed: {self.passed}")
        print(f"Failed: {self.failed}")
        
        if self.errors:
            print("\nFailed tests:")
            for error in self.errors:
                print(f"  {error}")
        
        print("=" * 80)
        return self.failed == 0


def start_http_server():
    """Start the HTTP server in the background"""
    global SERVER_PID
    
    print(f"🚀 Starting HTTP server on port {HTTP_PORT}...")
    
    # Use run_http_server.sh to start the server
    try:
        # Run the script which kills existing processes, builds, and starts the server
        result = subprocess.run(
            ["./run_http_server.sh"],
            capture_output=True,
            text=True,
            timeout=120  # 2 minutes for build + start
        )
        
        if result.returncode != 0:
            print(f"❌ Failed to start server: {result.stderr}")
            return False
        
        # Extract PID from output
        output_lines = result.stdout.split('\n')
        for line in output_lines:
            if "PID:" in line:
                try:
                    SERVER_PID = int(line.split("PID:")[1].strip())
                    print(f"✅ Server started with PID: {SERVER_PID}")
                except ValueError:
                    pass
        
        return True
        
    except subprocess.TimeoutExpired:
        print("❌ Timeout while starting server")
        return False
    except Exception as e:
        print(f"❌ Error starting server: {e}")
        return False


def wait_for_server_ready(timeout=SERVER_START_TIMEOUT):
    """Wait for the HTTP server to be ready"""
    print(f"⏳ Waiting for server to be ready (timeout: {timeout}s)...")
    
    start_time = time.time()
    while time.time() - start_time < timeout:
        try:
            result = subprocess.run(
                ["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", BASE_URL],
                capture_output=True,
                text=True,
                timeout=5
            )
            
            if result.returncode == 0 and result.stdout.strip() == "200":
                elapsed = time.time() - start_time
                print(f"✅ Server is ready (took {elapsed:.1f}s)")
                return True
        except Exception:
            pass
        
        time.sleep(1)
    
    print(f"❌ Server failed to become ready within {timeout}s")
    return False


def stop_http_server():
    """Stop the HTTP server"""
    global SERVER_PID
    
    print("\n🛑 Stopping HTTP server...")
    
    if SERVER_PID:
        try:
            os.kill(SERVER_PID, signal.SIGTERM)
            print(f"✅ Sent SIGTERM to PID {SERVER_PID}")
            time.sleep(2)
        except ProcessLookupError:
            print(f"⚠️  Process {SERVER_PID} not found")
        except Exception as e:
            print(f"⚠️  Error stopping server: {e}")
    
    # Note: run_http_server.sh already handles killing existing processes


def curl_get(endpoint):
    """Execute a curl GET request and return the response"""
    url = f"{BASE_URL}{endpoint}"
    
    cmd = [
        "curl", "-X", "GET", url,
        "-H", "Content-Type: application/json",
        "-s", "-w", "\n%{http_code}"
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        
        if result.returncode != 0:
            return None, None, f"Curl command failed: {result.stderr}"
        
        # Split response body and status code
        output = result.stdout.strip()
        parts = output.rsplit('\n', 1)
        
        if len(parts) != 2:
            return None, None, f"Invalid curl response format: {output}"
        
        response_body, status_code = parts
        status_code = int(status_code)
        
        # Parse JSON response if possible
        try:
            response_data = json.loads(response_body) if response_body else {}
        except json.JSONDecodeError:
            response_data = {"raw": response_body}
        
        return response_data, status_code, None
        
    except subprocess.TimeoutExpired:
        return None, None, "Request timeout"
    except Exception as e:
        return None, None, f"Error: {str(e)}"


def curl_post(endpoint, data=None, expected_status=200):
    """Execute a curl POST request and return the response"""
    url = f"{BASE_URL}{endpoint}"
    
    cmd = [
        "curl", "-X", "POST", url,
        "-H", "Content-Type: application/json",
        "-s", "-w", "\n%{http_code}"
    ]
    
    if data is not None:
        cmd.extend(["-d", json.dumps(data)])
    else:
        cmd.extend(["-d", "{}"])
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        
        if result.returncode != 0:
            return None, None, f"Curl command failed: {result.stderr}"
        
        # Split response body and status code
        output = result.stdout.strip()
        parts = output.rsplit('\n', 1)
        
        if len(parts) != 2:
            return None, None, f"Invalid curl response format: {output}"
        
        response_body, status_code = parts
        status_code = int(status_code)
        
        # Parse JSON response if possible
        try:
            response_data = json.loads(response_body) if response_body else {}
        except json.JSONDecodeError:
            response_data = {"raw": response_body}
        
        return response_data, status_code, None
        
    except subprocess.TimeoutExpired:
        return None, None, "Request timeout"
    except Exception as e:
        return None, None, f"Error: {str(e)}"


def get_available_schema_files():
    """Get list of schema files from available_schemas directory"""
    available_schemas_dir = "available_schemas"
    schema_files = []
    
    try:
        for filename in os.listdir(available_schemas_dir):
            if filename.endswith('.json'):
                # Extract schema name (filename without .json extension)
                schema_name = filename[:-5]
                schema_files.append(schema_name)
    except Exception as e:
        print(f"  ⚠️  Error reading available_schemas directory: {e}")
    
    return schema_files


def test_load_schemas(results):
    """Test: Load schemas from available_schemas directory"""
    print("\n" + "=" * 80)
    print("TEST 1: Load Schemas")
    print("=" * 80)
    
    response, status, error = curl_post("/api/schemas/load")
    
    if error:
        results.add_fail("Load schemas", error)
        return False
    
    if status != 200:
        results.add_fail("Load schemas", f"Expected status 200, got {status}")
        return False
    
    if not response or "data" not in response:
        results.add_fail("Load schemas", "Invalid response format")
        return False
    
    available_loaded = response["data"].get("available_schemas_loaded", 0)
    data_loaded = response["data"].get("data_schemas_loaded", 0)
    
    print(f"  Available schemas loaded: {available_loaded}")
    print(f"  Data schemas loaded: {data_loaded}")
    
    if available_loaded == 0 and data_loaded == 0:
        results.add_fail("Load schemas", "No schemas were loaded")
        return False
    
    results.add_pass("Load schemas")
    return True


def test_verify_schemas_available(results):
    """Test: Verify all schemas from available_schemas folder are discovered"""
    print("\n" + "=" * 80)
    print("TEST 2: Verify All Schemas Discovered")
    print("=" * 80)
    
    # Get list of schema files from the available_schemas directory
    expected_schemas = get_available_schema_files()
    print(f"  Expected schemas from available_schemas/: {', '.join(expected_schemas)}")
    
    if not expected_schemas:
        results.add_fail("Verify schemas discovered", "No schema files found in available_schemas/")
        return False
    
    # Query the API to get all schemas
    response, status, error = curl_get("/api/schemas")
    
    if error:
        results.add_fail("Verify schemas discovered", error)
        return False
    
    if status != 200:
        results.add_fail("Verify schemas discovered", f"Expected status 200, got {status}")
        return False
    
    if not response or "data" not in response:
        results.add_fail("Verify schemas discovered", "Invalid response format")
        print(f"  Response: {json.dumps(response, indent=2)}")
        return False
    
    # Parse the schemas data - API returns a list of schema objects
    schemas_data = response["data"]
    
    if not isinstance(schemas_data, list):
        results.add_fail("Verify schemas discovered", f"Expected list, got {type(schemas_data).__name__}")
        print(f"  Response: {json.dumps(response, indent=2)[:500]}...")
        return False
    
    # Extract schema names from the list
    discovered_schema_names = []
    for schema_obj in schemas_data:
        if isinstance(schema_obj, dict) and 'name' in schema_obj:
            discovered_schema_names.append(schema_obj['name'])
    
    print(f"  Discovered {len(discovered_schema_names)} schema(s) in database")
    
    # Verify each expected schema is present
    all_found = True
    for expected_name in expected_schemas:
        if expected_name in discovered_schema_names:
            # Find the schema object to get field count
            schema_obj = next((s for s in schemas_data if s.get('name') == expected_name), None)
            if schema_obj and isinstance(schema_obj, dict) and 'fields' in schema_obj:
                field_count = len(schema_obj['fields']) if isinstance(schema_obj['fields'], dict) else 0
                print(f"  ✅ {expected_name} (with {field_count} field(s))")
            else:
                print(f"  ✅ {expected_name} (loaded)")
        else:
            print(f"  ❌ {expected_name}: NOT FOUND")
            results.add_fail("Verify schemas discovered", 
                            f"Schema '{expected_name}' not found in API response")
            all_found = False
    
    if not all_found:
        print(f"\n  Discovered schemas: {', '.join(discovered_schema_names)}")
        print(f"  Missing schemas: {', '.join(set(expected_schemas) - set(discovered_schema_names))}")
        return False
    
    results.add_pass("Verify schemas discovered")
    return True


def test_approve_schema(results):
    """Test: Approve the BlogPost schema"""
    print("\n" + "=" * 80)
    print("TEST 3: Approve BlogPost Schema")
    print("=" * 80)
    
    response, status, error = curl_post("/api/schema/BlogPost/approve")
    
    if error:
        results.add_fail("Approve schema", error)
        return False
    
    if status != 200:
        results.add_fail("Approve schema", f"Expected status 200, got {status}")
        return False
    
    if not response or not response.get("success"):
        results.add_fail("Approve schema", "Schema approval failed")
        return False
    
    print("  BlogPost schema approved successfully")
    results.add_pass("Approve schema")
    return True


def test_create_mutation(results):
    """Test: Create a blog post mutation"""
    print("\n" + "=" * 80)
    print("TEST 4: Create Blog Post Mutation")
    print("=" * 80)
    
    # Create a test blog post with proper format
    publish_date = datetime.now().strftime("%Y-%m-%dT%H:%M:%SZ")
    
    mutation_data = {
        "type": "mutation",
        "schema": "BlogPost",
        "mutation_type": "create",
        "fields_and_values": {
            "title": "Integration Test Blog Post",
            "content": "This blog post was created by the integration test to verify the complete workflow of the DataFold HTTP API.",
            "author": "Integration Test Suite",
            "publish_date": publish_date,
            "tags": ["test", "integration", "automation"]
        },
        "key_value": {
            "hash": None,
            "range": publish_date  # Range key value for BlogPost schema
        }
    }
    
    print(f"  Creating blog post: {mutation_data['fields_and_values']['title']}")
    print(f"  Author: {mutation_data['fields_and_values']['author']}")
    print(f"  Publish date: {publish_date}")
    
    response, status, error = curl_post("/api/mutation", mutation_data)
    
    if error:
        results.add_fail("Create mutation", error)
        return False, None
    
    if status != 200:
        error_msg = response.get("error", "Unknown error") if response else "No response"
        results.add_fail("Create mutation", f"Expected status 200, got {status}: {error_msg}")
        print(f"  Response: {json.dumps(response, indent=2)}")
        return False, None
    
    # Check for success in the response data
    if response and (response.get("success") or response.get("data")):
        # Success could be either directly in response or in response.data
        is_success = response.get("success") or (isinstance(response.get("data"), dict) and response["data"].get("success"))
        
        if not is_success:
            error_msg = response.get("error", "Unknown error")
            results.add_fail("Create mutation", f"Mutation failed: {error_msg}")
            print(f"  Full response: {json.dumps(response, indent=2)}")
            return False, None
    else:
        error_msg = response.get("error", "Unknown error") if response else "No response"
        results.add_fail("Create mutation", f"Mutation failed: {error_msg}")
        print(f"  Full response: {json.dumps(response, indent=2)}")
        return False, None
    
    print("  Mutation created successfully")
    results.add_pass("Create mutation")
    
    # Return the publish_date for use in the query test
    return True, publish_date


def test_query_data(results, publish_date):
    """Test: Query the blog post data"""
    print("\n" + "=" * 80)
    print("TEST 5: Query Blog Post Data")
    print("=" * 80)
    
    query_data = {
        "type": "query",
        "schema": "BlogPost",
        "fields": ["title", "author", "publish_date", "tags", "content"]
    }
    
    print("  Querying all blog posts...")
    
    response, status, error = curl_post("/api/query", query_data)
    
    if error:
        results.add_fail("Query data", error)
        return False
    
    if status != 200:
        results.add_fail("Query data", f"Expected status 200, got {status}")
        return False
    
    # Check response format - could be in 'data' or 'results' field
    data = response.get("data") or response.get("results")
    
    if not data:
        results.add_fail("Query data", "No data returned from query")
        return False
    
    print(f"  Query returned {len(data) if isinstance(data, list) else 1} result(s)")
    
    # Search for our test post in the returned data
    found_test_post = False
    
    if isinstance(data, list):
        # Iterate through results to find our test post
        for item in data:
            if isinstance(item, dict) and 'fields' in item and 'key' in item:
                fields = item['fields']
                key = item['key']
                
                # Check if this is our test post by matching the range key (publish_date)
                if key.get('range') == publish_date:
                    found_test_post = True
                    print(f"  ✅ Found test blog post!")
                    print(f"  📝 Title: {fields.get('title', 'N/A')}")
                    print(f"  👤 Author: {fields.get('author', 'N/A')}")
                    print(f"  📅 Published: {key.get('range', 'N/A')}")
                    print(f"  🏷️  Tags: {', '.join(fields.get('tags', [])) if isinstance(fields.get('tags'), list) else str(fields.get('tags', 'N/A'))}")
                    
                    # Verify the data matches what we created
                    if fields.get('title') != "Integration Test Blog Post":
                        results.add_fail("Query data", "Title mismatch")
                        return False
                    if fields.get('author') != "Integration Test Suite":
                        results.add_fail("Query data", "Author mismatch")
                        return False
                    
                    break
    
    if not found_test_post:
        results.add_fail("Query data", f"Test post with publish_date {publish_date} not found in results")
        print(f"  Response structure: {json.dumps(data, indent=2)[:1000]}...")
        return False
    
    results.add_pass("Query data")
    return True


def run_integration_test():
    """Run the complete integration test"""
    results = TestResult()
    publish_date = None
    
    print("=" * 80)
    print("DataFold HTTP Server Integration Test")
    print("=" * 80)
    print(f"Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Base URL: {BASE_URL}")
    print("=" * 80)
    
    try:
        
        # Step 1: Start the HTTP server
        if not start_http_server():
            print("\n❌ Failed to start HTTP server. Aborting tests.")
            sys.exit(1)
        
        # Step 2: Wait for server to be ready
        if not wait_for_server_ready():
            print("\n❌ Server failed to become ready. Aborting tests.")
            sys.exit(1)
        
        # Give the server a moment to fully initialize
        time.sleep(2)
        
        # Step 3: Run tests
        if test_load_schemas(results):
            if test_verify_schemas_available(results):
                if test_approve_schema(results):
                    success, publish_date = test_create_mutation(results)
                    if success and publish_date:
                        test_query_data(results, publish_date)
        
    except KeyboardInterrupt:
        print("\n\n⚠️  Test interrupted by user")
    except Exception as e:
        print(f"\n\n❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        # Always stop the server
        stop_http_server()
    
    # Print summary and return exit code
    success = results.summary()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    run_integration_test()
