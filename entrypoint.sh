#!/bin/bash
# RegelRecht Upload Portal Entrypoint Script
# Verifies upload directory permissions before starting the application

set -e

UPLOAD_DIR="${UPLOAD_DIR:-/data}"

echo "RegelRecht Upload Portal starting..."
echo "Checking upload directory: $UPLOAD_DIR"

# Check if upload directory exists
if [ ! -d "$UPLOAD_DIR" ]; then
    echo "ERROR: Upload directory does not exist: $UPLOAD_DIR"
    echo "Creating directory..."
    mkdir -p "$UPLOAD_DIR" 2>/dev/null || {
        echo "ERROR: Failed to create upload directory"
        echo "Solution: Run as root: podman exec -u root <container> mkdir -p $UPLOAD_DIR && chown -R appuser:appuser $UPLOAD_DIR"
        exit 1
    }
fi

# Check if upload directory is writable
if [ ! -w "$UPLOAD_DIR" ]; then
    echo "ERROR: Upload directory is not writable: $UPLOAD_DIR"
    echo "Current user: $(whoami) (UID: $(id -u))"
    echo "Directory permissions:"
    ls -la "$UPLOAD_DIR" 2>/dev/null || ls -la "$(dirname "$UPLOAD_DIR")" 2>/dev/null || true
    echo ""
    echo "Solution: Run the following command to fix permissions:"
    echo "  podman exec -u root <container> chown -R appuser:appuser $UPLOAD_DIR"
    echo ""
    echo "Or for podman-compose deployments, add this to compose.yaml:"
    echo "  volumes:"
    echo "    - uploads:/data:U"
    echo ""
    exit 1
fi

# Test write capability
TEST_FILE="$UPLOAD_DIR/.write_test_$$"
if ! touch "$TEST_FILE" 2>/dev/null; then
    echo "ERROR: Cannot write to upload directory: $UPLOAD_DIR"
    echo "Write test failed"
    exit 1
fi
rm -f "$TEST_FILE" 2>/dev/null || true

echo "Upload directory check passed: $UPLOAD_DIR is writable"
echo "Starting application..."

# Execute the main application
exec /app/regelrecht-upload "$@"
