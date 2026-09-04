package main

import (
	"context"
	"testing"
	"time"
)

func TestMemoryTCPUDPICMP(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := selftest(ctx); err != nil {
		t.Fatal(err)
	}
}
