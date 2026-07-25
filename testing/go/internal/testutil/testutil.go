// Neo AGI OS — Go test utilities and context.

package testutil

import (
	"crypto/rand"
	"encoding/hex"
	"os"
	"path/filepath"
	"testing"
)

// TestContext provides a temporary directory and test configuration.
type TestContext struct {
	TmpDir  string
	Config  map[string]interface{}
	cleanup func()
}

// NewTestContext creates a TestContext with a temporary directory.
func NewTestContext(t *testing.T) *TestContext {
	t.Helper()
	tmpDir, err := os.MkdirTemp("", "neo-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	tc := &TestContext{
		TmpDir: tmpDir,
		Config: map[string]interface{}{
			"name":      "neo-test-config",
			"version":   "0.1.0",
			"debug":     true,
			"log_level": "debug",
		},
		cleanup: func() { os.RemoveAll(tmpDir) },
	}
	t.Cleanup(tc.Cleanup)
	return tc
}

// Cleanup removes the temporary directory.
func (tc *TestContext) Cleanup() {
	if tc.cleanup != nil {
		tc.cleanup()
		tc.cleanup = nil
	}
}

// TmpPath returns a path within the temporary directory.
func (tc *TestContext) TmpPath(name string) string {
	return filepath.Join(tc.TmpDir, name)
}

// RandomString generates a random hex string of the given byte length.
func RandomString(length int) string {
	b := make([]byte, length)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}
