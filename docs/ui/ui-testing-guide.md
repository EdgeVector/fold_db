# UI Testing Guide

This document provides instructions for testing the Datafold UI.

## Starting the Server

To run the web server for testing, execute the following command from the root of the repository:

```bash
./run_http_server.sh
```

This script will build the frontend and backend, and then start the server on `http://localhost:9001`.

You can then navigate to [http://localhost:9001](http://localhost:9001) in your browser to access the UI.

## Test Cases

### AI Agent Test: Key Registration

This test case is designed to be executed by an AI agent to verify the key management functionality.

**Objective**: Verify that a new Ed25519 key can be generated and registered as the system-wide public key.

**Steps**:

1.  **Navigate to the Key Management Page**: Access the application and navigate to the UI section responsible for security key management.
2.  **Generate a New Key**: Locate and click the "Generate Key" button. A new public/private key pair should be generated and displayed on the screen.
3.  **Register the Public Key**: Click the "Register Key" button to submit the newly generated public key to the server.
4.  **Verify Registration**:
    *   The UI should display a confirmation message indicating that the key was successfully registered.
    *   The displayed "Current System Public Key" should match the public key that was just generated. 