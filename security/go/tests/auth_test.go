// Neo Security — Go Tests
package auth_test

import (
    "testing"

    "github.com/neo-agi/neo/security/go/internal/auth"
)

func TestNewAuthService(t *testing.T) {
    svc := auth.NewAuthService()
    if svc == nil {
        t.Fatal("expected non-nil auth service")
    }
    if svc.ActiveTokens() != 0 {
        t.Fatal("expected 0 active tokens initially")
    }
}

func TestAuthenticateAndValidate(t *testing.T) {
    svc := auth.NewAuthService()

    creds := map[string]string{
        "api_key": "test-key-123",
    }
    token, err := svc.Authenticate(creds)
    if err != nil {
        t.Fatalf("failed to authenticate: %v", err)
    }
    if token == "" {
        t.Fatal("expected non-empty token")
    }

    valid, claims := svc.Validate(token)
    if !valid {
        t.Fatal("token should be valid")
    }
    if claims == nil {
        t.Fatal("expected non-nil claims")
    }
}
