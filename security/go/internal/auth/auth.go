package auth

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"sync"
	"time"
)

type AuthService struct {
	tokens sync.Map
	logger authLogger
}

type authLogger interface {
	Info(msg string, fields ...interface{})
	Error(msg string, fields ...interface{})
}

type noopLogger struct{}

func (n noopLogger) Info(msg string, fields ...interface{})  {}
func (n noopLogger) Error(msg string, fields ...interface{}) {}

type tokenEntry struct {
	Claims    map[string]string
	ExpiresAt time.Time
}

func NewAuthService() *AuthService {
	return &AuthService{
		logger: noopLogger{},
	}
}

func NewAuthServiceWithLogger(logger authLogger) *AuthService {
	return &AuthService{
		logger: logger,
	}
}

func (a *AuthService) Authenticate(credentials map[string]string) (string, error) {
	principal, ok := credentials["principal"]
	if !ok || principal == "" {
		return "", fmt.Errorf("missing principal in credentials")
	}

	token, err := generateToken()
	if err != nil {
		return "", fmt.Errorf("failed to generate token: %w", err)
	}

	claims := make(map[string]string)
	for k, v := range credentials {
		claims[k] = v
	}
	claims["authenticated_at"] = time.Now().Format(time.RFC3339)

	a.tokens.Store(token, &tokenEntry{
		Claims:    claims,
		ExpiresAt: time.Now().Add(24 * time.Hour),
	})

	a.logger.Info("authentication successful", "principal", principal)
	return token, nil
}

func (a *AuthService) Validate(token string) (bool, map[string]string) {
	val, ok := a.tokens.Load(token)
	if !ok {
		return false, nil
	}

	entry := val.(*tokenEntry)
	if time.Now().After(entry.ExpiresAt) {
		a.tokens.Delete(token)
		return false, nil
	}

	claims := make(map[string]string)
	for k, v := range entry.Claims {
		claims[k] = v
	}
	return true, claims
}

func (a *AuthService) Revoke(token string) error {
	_, loaded := a.tokens.LoadAndDelete(token)
	if !loaded {
		return fmt.Errorf("token not found")
	}
	a.logger.Info("token revoked")
	return nil
}

func (a *AuthService) ActiveTokens() int {
	count := 0
	a.tokens.Range(func(key, value interface{}) bool {
		entry := value.(*tokenEntry)
		if time.Now().Before(entry.ExpiresAt) {
			count++
		}
		return true
	})
	return count
}

func generateToken() (string, error) {
	b := make([]byte, 32)
	_, err := rand.Read(b)
	if err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}
