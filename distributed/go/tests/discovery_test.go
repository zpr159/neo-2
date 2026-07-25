// Neo Distributed — Go Tests
package discovery_test

import (
    "testing"

    "github.com/neo-agi/neo/distributed/go/internal/discovery"
)

func TestNewDiscovery(t *testing.T) {
    svc := discovery.NewDiscovery("static")
    if svc == nil {
        t.Fatal("expected non-nil discovery service")
    }
    if svc.NodeCount() != 0 {
        t.Fatal("expected 0 nodes initially")
    }
}

func TestRegisterDeregister(t *testing.T) {
    svc := discovery.NewDiscovery("static")

    info := map[string]string{
        "hostname": "node-1",
        "ip":       "192.168.1.1",
    }
    if err := svc.Register("node-1", info); err != nil {
        t.Fatalf("failed to register: %v", err)
    }
    if svc.NodeCount() != 1 {
        t.Fatal("expected 1 node after registration")
    }

    if err := svc.Deregister("node-1"); err != nil {
        t.Fatalf("failed to deregister: %v", err)
    }
    if svc.NodeCount() != 0 {
        t.Fatal("expected 0 nodes after deregistration")
    }
}
