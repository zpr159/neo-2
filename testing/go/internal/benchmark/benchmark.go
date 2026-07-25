// Neo AGI OS — Go benchmarking utilities.

package benchmark

import (
	"fmt"
	"sort"
	"time"
)

// BenchmarkCase represents a single benchmark test case.
type BenchmarkCase struct {
	Name string
	Fn   func()
}

// BenchmarkResult holds the result of a benchmark run.
type BenchmarkResult struct {
	Name     string
	Duration time.Duration
	Iterations int
}

// BenchmarkSuite manages and runs benchmark cases.
type BenchmarkSuite struct {
	cases   []BenchmarkCase
	results []BenchmarkResult
}

// NewSuite creates a new BenchmarkSuite.
func NewSuite() *BenchmarkSuite {
	return &BenchmarkSuite{}
}

// AddCase registers a benchmark case.
func (s *BenchmarkSuite) AddCase(name string, fn func()) {
	s.cases = append(s.cases, BenchmarkCase{Name: name, Fn: fn})
}

// RunAll executes all registered benchmark cases.
func (s *BenchmarkSuite) RunAll() []BenchmarkResult {
	s.results = make([]BenchmarkResult, 0, len(s.cases))
	for _, bc := range s.cases {
		iterations := 0
		start := time.Now()
		for elapsed := time.Duration(0); elapsed < time.Second; elapsed = time.Since(start) {
			bc.Fn()
			iterations++
		}
		duration := time.Since(start)
		s.results = append(s.results, BenchmarkResult{
			Name:       bc.Name,
			Duration:   duration,
			Iterations: iterations,
		})
	}
	return s.results
}

// Summary returns a formatted summary of all benchmark results.
func (s *BenchmarkSuite) Summary() string {
	if len(s.results) == 0 {
		return "No benchmark results."
	}

	sort.Slice(s.results, func(i, j int) bool {
		avgI := s.results[i].Duration / time.Duration(s.results[i].Iterations)
		avgJ := s.results[j].Duration / time.Duration(s.results[j].Iterations)
		return avgI < avgJ
	})

	summary := "Benchmark Results:\n"
	for _, r := range s.results {
		avg := r.Duration / time.Duration(r.Iterations)
		summary += fmt.Sprintf("  %-30s  %d iterations  avg: %s\n", r.Name, r.Iterations, avg)
	}
	return summary
}
