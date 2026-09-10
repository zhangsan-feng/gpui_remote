package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"sync"
	"time"
)

var tokenPattern = regexp.MustCompile(`Agent MCP bearer token:\s+(\S+)`)

type config struct {
	requests    int
	workers     int
	connections int
	timeout     time.Duration
	endpoint    string
	logDir      string
}

type result struct {
	ok      bool
	elapsed time.Duration
	error   string
}

type rpcResponse struct {
	Error  *rpcError  `json:"error"`
	Result *rpcResult `json:"result"`
}

type rpcError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

type rpcResult struct {
	IsError bool `json:"isError"`
}

func main() {
	cfg := config{}
	flag.IntVar(&cfg.requests, "requests", 2000, "number of MCP requests")
	flag.IntVar(&cfg.workers, "workers", 2000, "maximum number of concurrent requests")
	flag.IntVar(&cfg.connections, "connections", 128, "maximum number of TCP connections per host")
	flag.DurationVar(&cfg.timeout, "timeout", 20*time.Second, "per-request timeout")
	flag.StringVar(&cfg.endpoint, "endpoint", "http://127.0.0.1:37666/mcp", "MCP endpoint")
	flag.StringVar(&cfg.logDir, "log-dir", "logs", "directory containing application logs")
	flag.Parse()

	if cfg.requests <= 0 || cfg.workers <= 0 || cfg.connections <= 0 || cfg.timeout <= 0 {
		fatal("requests, workers, connections, and timeout must be positive")
	}

	token, err := latestToken(cfg.logDir)
	if err != nil {
		fatal(err.Error())
	}

	transport := &http.Transport{
		MaxIdleConns:        cfg.connections,
		MaxIdleConnsPerHost: cfg.connections,
		MaxConnsPerHost:     cfg.connections,
	}
	client := &http.Client{Transport: transport}

	jobs := make(chan int)
	results := make(chan result, cfg.requests)
	var workers sync.WaitGroup
	var ready sync.WaitGroup
	var start sync.WaitGroup
	ready.Add(cfg.workers)
	start.Add(1)

	for workerID := 0; workerID < cfg.workers; workerID++ {
		workers.Add(1)
		go func() {
			defer workers.Done()
			ready.Done()
			start.Wait()
			for requestID := range jobs {
				results <- callTool(client, cfg.endpoint, token, requestID, cfg.timeout)
			}
		}()
	}

	ready.Wait()
	startedAt := time.Now()
	go func() {
		start.Done()
		for requestID := 1; requestID <= cfg.requests; requestID++ {
			jobs <- requestID
		}
		close(jobs)
	}()

	workers.Wait()
	close(results)

	allResults := make([]result, 0, cfg.requests)
	for item := range results {
		allResults = append(allResults, item)
	}
	printReport(allResults, time.Since(startedAt))
	if failed := countFailures(allResults); failed > 0 {
		os.Exit(1)
	}
}

func latestToken(logDir string) (string, error) {
	entries, err := os.ReadDir(logDir)
	if err != nil {
		return "", fmt.Errorf("read log directory: %w", err)
	}

	type logFile struct {
		path    string
		modTime time.Time
	}
	files := make([]logFile, 0, len(entries))
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".log" {
			continue
		}
		info, statErr := entry.Info()
		if statErr != nil {
			continue
		}
		files = append(files, logFile{
			path:    filepath.Join(logDir, entry.Name()),
			modTime: info.ModTime(),
		})
	}
	sort.Slice(files, func(i, j int) bool {
		return files[i].modTime.After(files[j].modTime)
	})

	for _, file := range files {
		content, readErr := os.ReadFile(file.path)
		if readErr != nil {
			continue
		}
		matches := tokenPattern.FindAllSubmatch(content, -1)
		if len(matches) > 0 {
			return string(matches[len(matches)-1][1]), nil
		}
	}
	return "", errors.New("MCP bearer token not found in log files")
}

func callTool(client *http.Client, endpoint, token string, requestID int, timeout time.Duration) result {
	body, err := json.Marshal(map[string]any{
		"jsonrpc": "2.0",
		"id":      requestID,
		"method":  "tools/call",
		"params": map[string]any{
			"name":      "list_profiles",
			"arguments": map[string]any{},
		},
	})
	if err != nil {
		return result{error: "encode request: " + err.Error()}
	}

	request, err := http.NewRequest(http.MethodPost, endpoint, bytes.NewReader(body))
	if err != nil {
		return result{error: "create request: " + err.Error()}
	}
	request.Header.Set("Accept", "application/json, text/event-stream")
	request.Header.Set("Authorization", "Bearer "+token)
	request.Header.Set("Content-Type", "application/json")

	startedAt := time.Now()
	ctx, cancel := contextWithTimeout(timeout)
	defer cancel()
	request = request.WithContext(ctx)
	response, err := client.Do(request)
	elapsed := time.Since(startedAt)
	if err != nil {
		return result{elapsed: elapsed, error: classifyHTTPError(err)}
	}
	defer response.Body.Close()

	responseBody, readErr := io.ReadAll(response.Body)
	if readErr != nil {
		return result{elapsed: elapsed, error: "read response: " + readErr.Error()}
	}
	if response.StatusCode < http.StatusOK || response.StatusCode >= http.StatusMultipleChoices {
		return result{elapsed: elapsed, error: fmt.Sprintf("http %d", response.StatusCode)}
	}

	var payload rpcResponse
	if err := json.Unmarshal(responseBody, &payload); err != nil {
		return result{elapsed: elapsed, error: "invalid JSON response"}
	}
	if payload.Error != nil {
		return result{
			elapsed: elapsed,
			error:   fmt.Sprintf("rpc %d: %s", payload.Error.Code, payload.Error.Message),
		}
	}
	if payload.Result != nil && payload.Result.IsError {
		return result{elapsed: elapsed, error: "tool result error"}
	}
	return result{ok: true, elapsed: elapsed}
}

func contextWithTimeout(timeout time.Duration) (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), timeout)
}

func classifyHTTPError(err error) string {
	if errors.Is(err, context.DeadlineExceeded) {
		return "request timeout"
	}
	return "http request: " + err.Error()
}

func printReport(results []result, wallTime time.Duration) {
	durations := make([]float64, 0, len(results))
	errors := make(map[string]int)
	ok := 0
	for _, item := range results {
		durations = append(durations, float64(item.elapsed.Microseconds())/1000)
		if item.ok {
			ok++
		} else if item.error != "" {
			errors[item.error]++
		}
	}
	sort.Float64s(durations)
	failed := len(results) - ok
	fmt.Printf(
		"total=%d ok=%d failed=%d p50_ms=%.1f p95_ms=%.1f max_ms=%.1f wall_s=%.2f\n",
		len(results), ok, failed, percentile(durations, 0.50), percentile(durations, 0.95), durations[len(durations)-1], wallTime.Seconds(),
	)

	type errorCount struct {
		message string
		count   int
	}
	grouped := make([]errorCount, 0, len(errors))
	for message, count := range errors {
		grouped = append(grouped, errorCount{message: message, count: count})
	}
	sort.Slice(grouped, func(i, j int) bool { return grouped[i].count > grouped[j].count })
	for _, item := range grouped {
		fmt.Printf("error_count=%d error=%s\n", item.count, item.message)
	}
}

func percentile(values []float64, ratio float64) float64 {
	index := int(float64(len(values))*ratio) - 1
	if index < 0 {
		index = 0
	}
	if index >= len(values) {
		index = len(values) - 1
	}
	return values[index]
}

func countFailures(results []result) int {
	failed := 0
	for _, item := range results {
		if !item.ok {
			failed++
		}
	}
	return failed
}

func fatal(message string) {
	fmt.Fprintln(os.Stderr, message)
	os.Exit(2)
}
