package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

func main() {
	cfg := config{}
	flag.IntVar(&cfg.requests, "requests", 256, "number of list_profiles stress requests")
	flag.IntVar(&cfg.workers, "workers", 64, "maximum concurrent stress requests")
	flag.IntVar(&cfg.connections, "connections", 128, "maximum TCP connections per host")
	flag.IntVar(&cfg.concurrency, "concurrency", 16, "concurrent requests per workspace lane")
	flag.IntVar(&cfg.rounds, "rounds", 8, "sequential rounds in each workspace concurrency lane")
	flag.DurationVar(&cfg.timeout, "timeout", 20*time.Second, "per-request timeout")
	flag.StringVar(&cfg.endpoint, "endpoint", "http://127.0.0.1:37666/mcp", "MCP endpoint")
	flag.StringVar(&cfg.logDir, "log-dir", "logs", "directory containing application logs")
	flag.StringVar(&cfg.targetIP, "ip", targetIP, "target profile IP")
	flag.StringVar(&cfg.profileID, "profile-id", "", "optional profile id; defaults to the first profile matching -ip")
	flag.BoolVar(&cfg.stressOnly, "stress-only", false, "only run list_profiles stress")
	flag.BoolVar(&cfg.disableKeepAlives, "disable-keep-alives", false, "disable HTTP keep-alive connections for transport isolation")
	flag.Parse()

	if cfg.requests <= 0 || cfg.workers <= 0 || cfg.connections <= 0 || cfg.concurrency <= 0 || cfg.rounds <= 0 || cfg.timeout <= 0 {
		fatal("requests, workers, connections, concurrency, rounds, and timeout must be positive")
	}

	token, err := latestToken(cfg.logDir)
	if err != nil {
		fatal(err.Error())
	}
	logger, closeLog, err := newTestLogger(cfg.logDir)
	if err != nil {
		fatal(err.Error())
	}
	defer closeLog()

	transport := &http.Transport{
		MaxIdleConns:        cfg.connections,
		MaxIdleConnsPerHost: cfg.connections,
		MaxConnsPerHost:     cfg.connections,
		DisableKeepAlives:   cfg.disableKeepAlives,
	}
	api := &mcpClient{
		client:   &http.Client{Transport: transport},
		endpoint: cfg.endpoint,
		token:    token,
		timeout:  cfg.timeout,
		logger:   logger,
	}
	suite := &testSuite{api: api, cfg: cfg, logger: logger}
	startedAt := time.Now()
	logger.Printf("mcp_test_started endpoint=%s target_ip=%s requests=%d workers=%d connections=%d rounds=%d disable_keep_alives=%t", cfg.endpoint, cfg.targetIP, cfg.requests, cfg.workers, cfg.connections, cfg.rounds, cfg.disableKeepAlives)

	if cfg.stressOnly {
		runStress(suite)
	} else {
		runFullInterfaceSuite(suite)
		runStress(suite)
	}
	inspectApplicationLogs(cfg.logDir, startedAt, logger)
	suite.printReport(time.Since(startedAt))
	if suite.failureCount() > 0 {
		os.Exit(1)
	}
}

func newTestLogger(logDir string) (*log.Logger, func(), error) {
	if err := os.MkdirAll(logDir, 0o755); err != nil {
		return nil, nil, fmt.Errorf("create test log directory: %w", err)
	}
	path := filepath.Join(logDir, fmt.Sprintf("mcp-test-%s.log", time.Now().Format("20060102-150405.000")))
	file, err := os.Create(path)
	if err != nil {
		return nil, nil, fmt.Errorf("create test log: %w", err)
	}
	return log.New(io.MultiWriter(os.Stdout, file), "", log.LstdFlags|log.Lmicroseconds), func() { _ = file.Close() }, nil
}

func (api *mcpClient) call(method string, params any) (json.RawMessage, int, time.Duration, *rpcError, error) {
	requestID := api.nextID.Add(1)
	body, err := json.Marshal(map[string]any{
		"jsonrpc": "2.0",
		"id":      requestID,
		"method":  method,
		"params":  params,
	})
	if err != nil {
		return nil, 0, 0, nil, fmt.Errorf("encode request: %w", err)
	}
	request, err := http.NewRequest(http.MethodPost, api.endpoint, bytes.NewReader(body))
	if err != nil {
		return nil, 0, 0, nil, fmt.Errorf("create request: %w", err)
	}
	request.Header.Set("Accept", "application/json, text/event-stream")
	request.Header.Set("Authorization", "Bearer "+api.token)
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("MCP-Protocol-Version", protocolVersion)

	startedAt := time.Now()
	ctx, cancel := context.WithTimeout(context.Background(), api.timeout)
	defer cancel()
	request = request.WithContext(ctx)
	response, err := api.client.Do(request)
	elapsed := time.Since(startedAt)
	if err != nil {
		api.logger.Printf("mcp_http request_id=%d method=%s status=0 elapsed_ms=%d transport_error=%q", requestID, method, elapsed.Milliseconds(), err)
		return nil, 0, elapsed, nil, classifyHTTPError(err)
	}
	defer response.Body.Close()

	responseBody, readErr := io.ReadAll(response.Body)
	if readErr != nil {
		return nil, response.StatusCode, elapsed, nil, fmt.Errorf("read response: %w", readErr)
	}
	decoded, decodeErr := decodeResponseBody(responseBody)
	if decodeErr != nil {
		api.logger.Printf("mcp_http request_id=%d method=%s status=%d elapsed_ms=%d decode_error=%q body=%q", requestID, method, response.StatusCode, elapsed.Milliseconds(), decodeErr, compact(string(responseBody)))
		return nil, response.StatusCode, elapsed, nil, decodeErr
	}

	var envelope rpcEnvelope
	if len(decoded) > 0 {
		if err := json.Unmarshal(decoded, &envelope); err != nil {
			return nil, response.StatusCode, elapsed, nil, fmt.Errorf("decode JSON-RPC response: %w", err)
		}
	}
	if envelope.Error != nil {
		api.logger.Printf("mcp_http request_id=%d method=%s status=%d elapsed_ms=%d rpc_error=%d:%s", requestID, method, response.StatusCode, elapsed.Milliseconds(), envelope.Error.Code, envelope.Error.Message)
	} else {
		api.logger.Printf("mcp_http request_id=%d method=%s status=%d elapsed_ms=%d", requestID, method, response.StatusCode, elapsed.Milliseconds())
	}
	if response.StatusCode < http.StatusOK || response.StatusCode >= http.StatusMultipleChoices {
		return envelope.Result, response.StatusCode, elapsed, envelope.Error, fmt.Errorf("http %d: %s", response.StatusCode, compact(string(responseBody)))
	}
	return envelope.Result, response.StatusCode, elapsed, envelope.Error, nil
}

func (suite *testSuite) callTool(tool string, arguments map[string]any, expectToolError bool) toolOutcome {
	return suite.callToolInternal(tool, arguments, expectToolError, true)
}

func (suite *testSuite) callToolInternal(tool string, arguments map[string]any, expectToolError, recordFailure bool) toolOutcome {
	payload, status, elapsed, rpcErr, err := suite.api.call("tools/call", map[string]any{
		"name":      tool,
		"arguments": arguments,
	})
	outcome := toolOutcome{Tool: tool, HTTPStatus: status, Elapsed: elapsed, Payload: payload}
	if err != nil {
		outcome.Transport = err.Error()
	} else if rpcErr != nil {
		outcome.RPCError = fmt.Sprintf("rpc %d: %s", rpcErr.Code, rpcErr.Message)
	} else {
		var result toolCallResult
		if decodeErr := json.Unmarshal(payload, &result); decodeErr != nil {
			outcome.Transport = fmt.Sprintf("decode tool result: %v", decodeErr)
		} else if result.IsError {
			outcome.ToolError = toolErrorText(result)
		}
	}
	unexpected := outcome.Transport != "" || outcome.RPCError != "" || (expectToolError && outcome.ToolError == "") || (!expectToolError && outcome.ToolError != "")
	if unexpected && recordFailure {
		suite.recordToolFailure(outcome, expectToolError)
	}
	suite.logger.Printf("mcp_tool tool=%s elapsed_ms=%d status=%d expected_error=%t transport_error=%q rpc_error=%q tool_error=%q", tool, elapsed.Milliseconds(), status, expectToolError, outcome.Transport, outcome.RPCError, outcome.ToolError)
	return outcome
}

func toolErrorText(result toolCallResult) string {
	for _, item := range result.Content {
		if item.Text != "" {
			return item.Text
		}
	}
	return "tool returned isError=true"
}

func (suite *testSuite) recordToolFailure(outcome toolOutcome, expectedError bool) {
	reason := outcome.Transport
	if reason == "" {
		reason = outcome.RPCError
	}
	if reason == "" && expectedError && outcome.ToolError == "" {
		reason = "expected tool error but call succeeded"
	}
	if reason == "" && !expectedError && outcome.ToolError != "" {
		reason = "unexpected tool error: " + outcome.ToolError
	}
	suite.fail("tool=%s status=%d elapsed_ms=%d reason=%s", outcome.Tool, outcome.HTTPStatus, outcome.Elapsed.Milliseconds(), reason)
}

func (suite *testSuite) fail(format string, args ...any) {
	message := fmt.Sprintf(format, args...)
	suite.mu.Lock()
	suite.failures = append(suite.failures, message)
	suite.mu.Unlock()
	suite.logger.Printf("mcp_test_failure %s", message)
}

func (suite *testSuite) failureCount() int {
	suite.mu.Lock()
	defer suite.mu.Unlock()
	return len(suite.failures)
}

func (suite *testSuite) printReport(wallTime time.Duration) {
	suite.mu.Lock()
	failures := append([]string(nil), suite.failures...)
	suite.mu.Unlock()
	suite.logger.Printf("mcp_test_summary failures=%d wall_ms=%d", len(failures), wallTime.Milliseconds())
	for _, failure := range failures {
		suite.logger.Printf("mcp_test_failure_summary %s", failure)
	}
}

func runFullInterfaceSuite(suite *testSuite) {
	suite.logger.Printf("mcp_interface_suite_started tool_count=%d", len(expectedTools))
	tools := listTools(suite)
	missing := make(map[string]bool)
	for _, expected := range expectedTools {
		missing[expected] = true
	}
	for _, name := range tools {
		delete(missing, name)
	}
	for name := range missing {
		suite.fail("missing MCP tool: %s", name)
	}
	if len(tools) != len(expectedTools) {
		suite.logger.Printf("mcp_tool_inventory expected=%d actual=%d tools=%v", len(expectedTools), len(tools), tools)
	}

	profilesRaw := suite.callTool("list_profiles", map[string]any{}, false)
	profiles := parseProfiles(profilesRaw.Payload)
	profile := chooseProfile(profiles, suite.cfg.targetIP, suite.cfg.profileID)
	if profile.ID == "" {
		suite.fail("no saved profile matched ip=%s profile_id=%s", suite.cfg.targetIP, suite.cfg.profileID)
		return
	}
	suite.logger.Printf("mcp_target_profile profile_id=%s title=%s ip=%s protocol=%s", profile.ID, profile.Title, profile.IP, profile.Protocol)
	suite.callTool("list_sftp_sessions", map[string]any{}, false)
	suite.callTool("list_terminals", map[string]any{}, false)

	sshWorkspace := openWorkspace(suite, profile, "ssh")
	sftpWorkspace := openWorkspace(suite, profile, "sftp")
	if sshWorkspace == "" || sftpWorkspace == "" {
		if sshWorkspace != "" {
			suite.callTool("close_session", map[string]any{"workspace_id": sshWorkspace}, false)
		}
		if sftpWorkspace != "" {
			suite.callTool("close_session", map[string]any{"workspace_id": sftpWorkspace}, false)
		}
		return
	}
	defer func() {
		suite.callTool("close_session", map[string]any{"workspace_id": sftpWorkspace}, false)
		suite.callTool("close_session", map[string]any{"workspace_id": sshWorkspace}, false)
	}()

	localDir, err := os.MkdirTemp("", "gpui-mcp-interface-")
	if err != nil {
		suite.fail("create local interface test directory: %v", err)
		return
	}
	defer os.RemoveAll(localDir)
	localFile := filepath.Join(localDir, "mcp-interface-probe.txt")
	if err := os.WriteFile(localFile, []byte("mcp interface probe\n"), 0o600); err != nil {
		suite.fail("create local interface test file: %v", err)
		return
	}

	suite.callTool("read_terminal", map[string]any{"workspace_id": sshWorkspace, "offset": 0, "limit": 20}, false)
	suite.callTool("send_text", map[string]any{"workspace_id": sshWorkspace, "text": ""}, false)
	suite.callTool("send_key", map[string]any{"workspace_id": sshWorkspace, "key": "space"}, false)

	suite.callTool("list_sftp_local", map[string]any{"workspace_id": sftpWorkspace}, false)
	suite.callTool("change_sftp_local_directory", map[string]any{
		"workspace_id": sftpWorkspace,
		"ip":           profile.IP,
		"title":        profile.Title,
		"path":         localDir,
	}, false)
	suite.callTool("list_sftp_local", map[string]any{"workspace_id": sftpWorkspace}, false)

	// SFTP opens the first remote directory asynchronously. Wait until the
	// application has published a remote path before exercising watch/transfer
	// commands that depend on it.
	remoteOutcome, remote, remoteReady := waitForRemoteDirectory(suite, sftpWorkspace)
	remotePath := remote.Path
	if remotePath == "" {
		remotePath = "."
	}

	watchOutcome := suite.callTool("watch_sftp_local", map[string]any{
		"workspace_id": sftpWorkspace,
		"ip":           profile.IP,
		"title":        profile.Title,
		"local_path":   localFile,
	}, !remoteReady)
	suite.callTool("list_sftp_local_watches", map[string]any{
		"workspace_id": sftpWorkspace,
		"ip":           profile.IP,
		"title":        profile.Title,
	}, false)
	suite.callTool("stop_sftp_local_watch", map[string]any{
		"workspace_id": sftpWorkspace,
		"ip":           profile.IP,
		"title":        profile.Title,
		"local_path":   localFile,
	}, !remoteReady || watchOutcome.ToolError != "")
	suite.callTool("list_sftp_local_watches", map[string]any{
		"workspace_id": sftpWorkspace,
		"ip":           profile.IP,
		"title":        profile.Title,
	}, false)

	_ = remoteOutcome
	suite.callTool("change_sftp_remote_directory", map[string]any{
		"workspace_id": sftpWorkspace,
		"ip":           profile.IP,
		"title":        profile.Title,
		"path":         remotePath,
	}, false)

	// Queueing a missing local file exercises upload_sftp without writing a real
	// file to the remote host. The transfer is intentionally checked afterwards.
	suite.callTool("upload_sftp", map[string]any{
		"workspace_id": sftpWorkspace,
		"local_paths":  []string{filepath.Join(localDir, "missing-upload-probe.txt")},
	}, false)
	if entry := firstDownloadableEntry(remote.Entries); entry.Path != "" {
		suite.callTool("download_sftp", map[string]any{
			"workspace_id": sftpWorkspace,
			"remote_paths": []string{entry.Path},
		}, false)
	} else {
		suite.callTool("download_sftp", map[string]any{
			"workspace_id": sftpWorkspace,
			"remote_paths": []string{remotePath + "/__mcp_missing_download_probe__"},
		}, true)
	}
	suite.callTool("list_sftp_transfers", map[string]any{"workspace_id": sftpWorkspace}, false)
	runWorkspaceConcurrency(suite, sshWorkspace, sftpWorkspace)
	suite.logger.Printf("mcp_interface_suite_finished ssh_workspace=%s sftp_workspace=%s", sshWorkspace, sftpWorkspace)
}

func listTools(suite *testSuite) []string {
	payload, status, elapsed, rpcErr, err := suite.api.call("tools/list", map[string]any{})
	if err != nil {
		suite.fail("tools/list status=%d elapsed_ms=%d: %v", status, elapsed.Milliseconds(), err)
		return nil
	}
	if rpcErr != nil {
		suite.fail("tools/list rpc error: %d:%s", rpcErr.Code, rpcErr.Message)
		return nil
	}
	var response struct {
		Tools []struct {
			Name string `json:"name"`
		} `json:"tools"`
	}
	if err := json.Unmarshal(payload, &response); err != nil {
		suite.fail("decode tools/list result: %v", err)
		return nil
	}
	tools := make([]string, 0, len(response.Tools))
	for _, tool := range response.Tools {
		tools = append(tools, tool.Name)
	}
	sort.Strings(tools)
	suite.logger.Printf("mcp_tool_inventory status=%d elapsed_ms=%d tools=%v", status, elapsed.Milliseconds(), tools)
	return tools
}

func openWorkspace(suite *testSuite, profile profileSummary, protocol string) string {
	outcome := suite.callTool("open_session", map[string]any{
		"profile_id": profile.ID,
		"ip":         profile.IP,
		"title":      profile.Title,
		"protocol":   protocol,
	}, false)
	if outcome.ToolError != "" || outcome.Transport != "" || outcome.RPCError != "" {
		return ""
	}
	payload, err := toolPayload(outcome.Payload)
	if err != nil {
		suite.fail("decode open_session(%s) output: %v", protocol, err)
		return ""
	}
	var output openSessionOutput
	if err := json.Unmarshal(payload, &output); err != nil || output.WorkspaceID == "" {
		suite.fail("invalid open_session(%s) output: %v payload=%s", protocol, err, compact(string(payload)))
		return ""
	}
	suite.logger.Printf("mcp_workspace_opened protocol=%s workspace_id=%s", protocol, output.WorkspaceID)
	return output.WorkspaceID
}

func waitForRemoteDirectory(suite *testSuite, workspaceID string) (toolOutcome, sftpDirectoryOutput, bool) {
	deadline := time.Now().Add(suite.cfg.timeout)
	var outcome toolOutcome
	var remote sftpDirectoryOutput
	for attempt := 1; ; attempt++ {
		outcome = suite.callToolInternal("list_sftp_remote", map[string]any{"workspace_id": workspaceID}, false, false)
		remote = parseSftpDirectory(outcome.Payload)
		if outcome.ToolError == "" && outcome.Transport == "" && outcome.RPCError == "" && remote.Path != "" {
			suite.logger.Printf("mcp_sftp_remote_ready workspace_id=%s path=%s attempts=%d", workspaceID, remote.Path, attempt)
			return outcome, remote, true
		}
		suite.logger.Printf(
			"mcp_sftp_remote_wait workspace_id=%s attempt=%d path=%s transport_error=%q rpc_error=%q tool_error=%q",
			workspaceID,
			attempt,
			remote.Path,
			outcome.Transport,
			outcome.RPCError,
			outcome.ToolError,
		)
		if time.Now().After(deadline) {
			break
		}
		time.Sleep(500 * time.Millisecond)
	}
	if outcome.Transport != "" || outcome.RPCError != "" || outcome.ToolError != "" {
		suite.recordToolFailure(outcome, false)
	} else {
		suite.fail("remote directory was not ready before timeout: workspace_id=%s timeout=%s", workspaceID, suite.cfg.timeout)
	}
	return outcome, remote, false
}

func retryTool(suite *testSuite, tool string, arguments map[string]any, attempts int, delay time.Duration) toolOutcome {
	var outcome toolOutcome
	for attempt := 1; attempt <= attempts; attempt++ {
		outcome = suite.callToolInternal(tool, arguments, false, false)
		if outcome.ToolError == "" && outcome.Transport == "" && outcome.RPCError == "" {
			return outcome
		}
		suite.logger.Printf("mcp_tool_retry tool=%s attempt=%d/%d error=%q", tool, attempt, attempts, outcome.ToolError+outcome.Transport+outcome.RPCError)
		time.Sleep(delay)
	}
	suite.recordToolFailure(outcome, false)
	return outcome
}

func runWorkspaceConcurrency(suite *testSuite, sshWorkspace, sftpWorkspace string) {
	type lane struct {
		name      string
		workspace string
		tool      string
		arguments map[string]any
	}
	lanes := []lane{
		{name: "ssh", workspace: sshWorkspace, tool: "read_terminal", arguments: map[string]any{"workspace_id": sshWorkspace, "offset": 0, "limit": 20}},
		{name: "sftp", workspace: sftpWorkspace, tool: "list_sftp_local", arguments: map[string]any{"workspace_id": sftpWorkspace}},
	}

	start := make(chan struct{})
	var wg sync.WaitGroup
	var totalNanos atomic.Int64
	var completed atomic.Int64
	startedAt := time.Now()
	for _, currentLane := range lanes {
		currentLane := currentLane
		wg.Add(1)
		go func() {
			defer wg.Done()
			<-start
			for round := 0; round < suite.cfg.rounds; round++ {
				outcome := suite.callTool(currentLane.tool, currentLane.arguments, false)
				totalNanos.Add(outcome.Elapsed.Nanoseconds())
				completed.Add(1)
				suite.logger.Printf("mcp_workspace_concurrency lane=%s workspace_id=%s round=%d elapsed_ms=%d", currentLane.name, currentLane.workspace, round+1, outcome.Elapsed.Milliseconds())
			}
		}()
	}
	close(start)
	wg.Wait()
	wall := time.Since(startedAt)
	if wall > 0 {
		ratio := float64(totalNanos.Load()) / float64(wall.Nanoseconds())
		suite.logger.Printf("mcp_workspace_concurrency_summary lanes=%d completed=%d wall_ms=%d summed_request_ms=%.1f overlap_ratio=%.2f", len(lanes), completed.Load(), wall.Milliseconds(), float64(totalNanos.Load())/1e6, ratio)
	}

	var controlWG sync.WaitGroup
	for i := 0; i < suite.cfg.concurrency; i++ {
		controlWG.Add(1)
		go func() {
			defer controlWG.Done()
			suite.callTool("list_profiles", map[string]any{}, false)
		}()
	}
	controlWG.Wait()
}

func runStress(suite *testSuite) {
	suite.logger.Printf("mcp_stress_started requests=%d workers=%d", suite.cfg.requests, suite.cfg.workers)
	jobs := make(chan int)
	results := make(chan toolOutcome, suite.cfg.requests)
	var workers sync.WaitGroup
	var ready sync.WaitGroup
	start := make(chan struct{})
	ready.Add(suite.cfg.workers)
	for workerID := 0; workerID < suite.cfg.workers; workerID++ {
		workers.Add(1)
		go func() {
			defer workers.Done()
			ready.Done()
			<-start
			for range jobs {
				results <- suite.callTool("list_profiles", map[string]any{}, false)
			}
		}()
	}
	ready.Wait()
	startedAt := time.Now()
	close(start)
	go func() {
		for requestID := 1; requestID <= suite.cfg.requests; requestID++ {
			jobs <- requestID
		}
		close(jobs)
	}()
	workers.Wait()
	close(results)

	var durations []float64
	ok := 0
	for outcome := range results {
		durations = append(durations, float64(outcome.Elapsed.Microseconds())/1000)
		if outcome.Transport == "" && outcome.RPCError == "" && outcome.ToolError == "" {
			ok++
		}
	}
	sort.Float64s(durations)
	if len(durations) == 0 {
		suite.fail("mcp stress returned no results")
		return
	}
	failed := len(durations) - ok
	suite.logger.Printf("mcp_stress_summary total=%d ok=%d failed=%d p50_ms=%.1f p95_ms=%.1f max_ms=%.1f wall_ms=%d", len(durations), ok, failed, percentile(durations, 0.50), percentile(durations, 0.95), durations[len(durations)-1], time.Since(startedAt).Milliseconds())
}

func parseProfiles(raw json.RawMessage) []profileSummary {
	payload, err := toolPayload(raw)
	if err != nil {
		return nil
	}
	var profiles []profileSummary
	if json.Unmarshal(payload, &profiles) != nil {
		return nil
	}
	return profiles
}

func chooseProfile(profiles []profileSummary, ip, profileID string) profileSummary {
	for _, profile := range profiles {
		if profileID != "" && profile.ID == profileID {
			return profile
		}
	}
	for _, profile := range profiles {
		profileIP := profile.IP
		if profileIP == "" {
			profileIP = profile.Host
		}
		if profileIP == ip || profile.Host == ip {
			if profile.IP == "" {
				profile.IP = profile.Host
			}
			return profile
		}
	}
	return profileSummary{}
}

func parseSftpDirectory(raw json.RawMessage) sftpDirectoryOutput {
	payload, err := toolPayload(raw)
	if err != nil {
		return sftpDirectoryOutput{}
	}
	var directory sftpDirectoryOutput
	if json.Unmarshal(payload, &directory) != nil {
		return sftpDirectoryOutput{}
	}
	return directory
}

func firstDownloadableEntry(entries []sftpEntryOutput) sftpEntryOutput {
	for _, entry := range entries {
		if !entry.IsDirectory {
			return entry
		}
	}
	return sftpEntryOutput{}
}

func toolPayload(raw json.RawMessage) (json.RawMessage, error) {
	var result toolCallResult
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, err
	}
	if len(result.StructuredContent) > 0 && string(result.StructuredContent) != "null" {
		return result.StructuredContent, nil
	}
	for _, item := range result.Content {
		if item.Type == "text" && json.Valid([]byte(item.Text)) {
			return json.RawMessage(item.Text), nil
		}
	}
	return nil, errors.New("tool result has no structured JSON payload")
}

func decodeResponseBody(body []byte) ([]byte, error) {
	body = bytes.TrimSpace(body)
	if len(body) == 0 {
		return nil, nil
	}
	if json.Valid(body) {
		return body, nil
	}
	scanner := bufio.NewScanner(bytes.NewReader(body))
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "data:") {
			data := bytes.TrimSpace([]byte(strings.TrimSpace(strings.TrimPrefix(line, "data:"))))
			if len(data) > 0 && json.Valid(data) {
				return data, nil
			}
		}
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	return nil, errors.New("response is neither JSON nor SSE JSON")
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
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".log" || strings.HasPrefix(entry.Name(), "mcp-test-") {
			continue
		}
		info, statErr := entry.Info()
		if statErr != nil {
			continue
		}
		files = append(files, logFile{path: filepath.Join(logDir, entry.Name()), modTime: info.ModTime()})
	}
	sort.Slice(files, func(i, j int) bool { return files[i].modTime.After(files[j].modTime) })
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

func inspectApplicationLogs(logDir string, startedAt time.Time, logger *log.Logger) {
	entries, err := os.ReadDir(logDir)
	if err != nil {
		logger.Printf("mcp_app_log_inspection_error=%q", err)
		return
	}
	keywords := []string{"MCP HTTP request", "MCP tool handler", "MCP bridge", "MCP application bridge", "queue full", "application error", "lane stopping", "response receiver dropped"}
	startText := startedAt.Format("2006-01-02 15:04:05")
	matched := 0
	counts := make(map[string]int)
	var recent []string
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".log" || strings.HasPrefix(entry.Name(), "mcp-test-") {
			continue
		}
		file, openErr := os.Open(filepath.Join(logDir, entry.Name()))
		if openErr != nil {
			continue
		}
		scanner := bufio.NewScanner(file)
		for scanner.Scan() {
			line := scanner.Text()
			if len(line) < 21 || strings.Compare(line[1:20], startText) < 0 {
				continue
			}
			for _, keyword := range keywords {
				if strings.Contains(line, keyword) {
					matched++
					counts[keyword]++
					recent = append(recent, line)
					if len(recent) > 40 {
						recent = recent[len(recent)-40:]
					}
					break
				}
			}
		}
		_ = file.Close()
	}
	logger.Printf("mcp_app_log_summary matched=%d counts=%v", matched, counts)
	for _, line := range recent {
		logger.Printf("mcp_app_log %s", line)
	}
}

func classifyHTTPError(err error) error {
	if errors.Is(err, context.DeadlineExceeded) {
		return errors.New("request timeout")
	}
	return fmt.Errorf("http request: %w", err)
}

func compact(value string) string {
	value = strings.Join(strings.Fields(value), " ")
	if len(value) > 500 {
		return value[:500] + "..."
	}
	return value
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

func fatal(message string) {
	fmt.Fprintln(os.Stderr, message)
	os.Exit(2)
}
