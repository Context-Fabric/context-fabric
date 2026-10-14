"""Integration tests for MCP server transport modes.

Tests that the server can start with all three transport modes:
- STDIO (default)
- SSE (for Cursor, remote clients)
- HTTP (streamable HTTP for production)
"""

import subprocess
import sys
import time

import pytest
import requests


@pytest.fixture
def mini_corpus_path(fixtures_dir):
    """Path to minimal test corpus (a temporary copy; see ``fixtures_dir``)."""
    return str(fixtures_dir / "mini_corpus")


@pytest.fixture
def cfabric_mcp_command():
    """Get the cfabric-mcp command path."""
    # Use the installed command from the virtual environment
    return [sys.executable, "-m", "cfabric_mcp.server"]


class TestStdioTransport:
    """Tests for STDIO transport mode."""

    def test_stdio_starts_without_error(self, mini_corpus_path, cfabric_mcp_command):
        """Test that server starts with STDIO transport without error.

        STDIO transport blocks waiting for input, so we just verify it starts
        successfully by checking stderr for the ready message.
        """
        proc = subprocess.Popen(
            cfabric_mcp_command + ["--corpus", mini_corpus_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            # Give it time to start and load corpus
            time.sleep(2)

            # Check if process is still running (good) or crashed (bad)
            poll_result = proc.poll()

            # Read any stderr output
            proc.stdin.close()  # Close stdin to signal EOF
            stderr_output = proc.stderr.read().decode("utf-8", errors="replace")

            # Process should either still be running or have exited cleanly
            # If it crashed with TypeError, poll_result will be non-zero
            if poll_result is not None and poll_result != 0:
                pytest.fail(f"Server crashed with exit code {poll_result}. Stderr: {stderr_output}")

            # Verify we see the startup message
            assert "Server ready with corpora" in stderr_output
        finally:
            proc.terminate()
            proc.wait(timeout=5)


class TestSseTransport:
    """Tests for SSE transport mode."""

    def test_sse_transport_starts_without_error(self, mini_corpus_path, cfabric_mcp_command):
        """Test that server starts with SSE transport without TypeError.

        This test verifies the bug fix: mcp.run() should not receive
        host/port arguments that it doesn't accept.
        """
        port = 18765  # Use high port to avoid conflicts

        proc = subprocess.Popen(
            cfabric_mcp_command + ["--corpus", mini_corpus_path, "--sse", str(port)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            # Give it time to start
            time.sleep(2)

            # Check if process crashed
            poll_result = proc.poll()
            stderr_output = proc.stderr.read().decode("utf-8", errors="replace") if poll_result else ""

            if poll_result is not None and poll_result != 0:
                pytest.fail(f"SSE server crashed with exit code {poll_result}. Stderr: {stderr_output}")

            # Process should still be running if it started successfully
            assert poll_result is None, f"Server exited unexpectedly. Stderr: {stderr_output}"

        finally:
            proc.terminate()
            proc.wait(timeout=5)

    def test_sse_endpoint_accessible(self, mini_corpus_path, cfabric_mcp_command):
        """Test that SSE endpoint is accessible after server starts."""
        port = 18766

        proc = subprocess.Popen(
            cfabric_mcp_command + ["--corpus", mini_corpus_path, "--sse", str(port)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            # Give it time to start and bind
            time.sleep(3)

            # Check process is running
            if proc.poll() is not None:
                stderr = proc.stderr.read().decode("utf-8", errors="replace")
                pytest.fail(f"Server crashed before we could test endpoint. Stderr: {stderr}")

            # Try to connect to the SSE endpoint
            try:
                response = requests.get(f"http://127.0.0.1:{port}/sse", timeout=2, stream=True)
                # SSE endpoint should return 200 with event-stream content type
                assert response.status_code == 200
                assert "text/event-stream" in response.headers.get("content-type", "")
            except requests.exceptions.ConnectionError:
                pytest.fail("Could not connect to SSE endpoint")

        finally:
            proc.terminate()
            proc.wait(timeout=5)


class TestHttpTransport:
    """Tests for HTTP (streamable-http) transport mode."""

    def test_http_transport_starts_without_error(self, mini_corpus_path, cfabric_mcp_command):
        """Test that server starts with HTTP transport without errors.

        This test verifies two bug fixes:
        1. mcp.run() should not receive host/port arguments
        2. Transport name should be 'streamable-http' not 'http'
        """
        port = 18767

        proc = subprocess.Popen(
            cfabric_mcp_command + ["--corpus", mini_corpus_path, "--http", str(port)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            # Give it time to start
            time.sleep(2)

            # Check if process crashed
            poll_result = proc.poll()
            stderr_output = proc.stderr.read().decode("utf-8", errors="replace") if poll_result else ""

            if poll_result is not None and poll_result != 0:
                pytest.fail(f"HTTP server crashed with exit code {poll_result}. Stderr: {stderr_output}")

            # Process should still be running
            assert poll_result is None, f"Server exited unexpectedly. Stderr: {stderr_output}"

        finally:
            proc.terminate()
            proc.wait(timeout=5)

    def test_http_endpoint_accessible(self, mini_corpus_path, cfabric_mcp_command):
        """Test that HTTP endpoint is accessible after server starts."""
        port = 18768

        proc = subprocess.Popen(
            cfabric_mcp_command + ["--corpus", mini_corpus_path, "--http", str(port)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            # Give it time to start and bind
            time.sleep(3)

            # Check process is running
            if proc.poll() is not None:
                stderr = proc.stderr.read().decode("utf-8", errors="replace")
                pytest.fail(f"Server crashed before we could test endpoint. Stderr: {stderr}")

            # Try to connect to the HTTP endpoint
            try:
                response = requests.get(f"http://127.0.0.1:{port}/mcp", timeout=2)
                # HTTP endpoint exists and responds
                # 405 = Method Not Allowed (expects POST), 406 = Not Acceptable (missing Accept header)
                assert response.status_code in (200, 400, 405, 406)
            except requests.exceptions.ConnectionError:
                pytest.fail("Could not connect to HTTP endpoint")

        finally:
            proc.terminate()
            proc.wait(timeout=5)


class TestCustomHostPort:
    """Tests for custom host/port configuration."""

    def test_sse_custom_port(self, mini_corpus_path, cfabric_mcp_command):
        """Test SSE transport binds to custom port."""
        port = 18769

        proc = subprocess.Popen(
            cfabric_mcp_command + ["--corpus", mini_corpus_path, "--sse", str(port)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            time.sleep(3)

            if proc.poll() is not None:
                stderr = proc.stderr.read().decode("utf-8", errors="replace")
                pytest.fail(f"Server crashed. Stderr: {stderr}")

            # Verify server is listening on the specified port
            try:
                response = requests.get(f"http://127.0.0.1:{port}/sse", timeout=2, stream=True)
                assert response.status_code == 200
            except requests.exceptions.ConnectionError:
                pytest.fail(f"Server not listening on port {port}")

        finally:
            proc.terminate()
            proc.wait(timeout=5)

    def test_http_custom_host(self, mini_corpus_path, cfabric_mcp_command):
        """Test HTTP transport with custom host binding."""
        port = 18770

        proc = subprocess.Popen(
            cfabric_mcp_command
            + ["--corpus", mini_corpus_path, "--http", str(port), "--host", "127.0.0.1"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            time.sleep(3)

            if proc.poll() is not None:
                stderr = proc.stderr.read().decode("utf-8", errors="replace")
                pytest.fail(f"Server crashed. Stderr: {stderr}")

            # Verify server is accessible on 127.0.0.1
            try:
                response = requests.get(f"http://127.0.0.1:{port}/mcp", timeout=2)
                assert response.status_code in (200, 400, 405, 406)
            except requests.exceptions.ConnectionError:
                pytest.fail("Server not accessible on 127.0.0.1")

        finally:
            proc.terminate()
            proc.wait(timeout=5)
