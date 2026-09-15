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
	"regexp"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

const protocolVersion = "2025-03-26"

type scenarioConfig struct {
	endpoint  string
	logDir    string
	targetIP  string
	profileID string
	timeout   time.Duration
	rounds    int
}

func bindCommonFlags(cfg *scenarioConfig) {
	flag.StringVar(&cfg.endpoint, "endpoint", "http://127.0.0.1:37666/mcp", "MCP endpoint")
	flag.StringVar(&cfg.logDir, "log-dir", "logs", "directory containing application logs")
	flag.StringVar(&cfg.targetIP, "ip", "", "target profile IP")
	flag.StringVar(&cfg.profileID, "profile-id", "", "optional profile id")
	flag.DurationVar(&cfg.timeout, "timeout", 20*time.Second, "per-request timeout")
}

type rpcError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

type rpcEnvelope struct {
	Error  *rpcError       `json:"error"`
	Result json.RawMessage `json:"result"`
}

type toolCallResult struct {
	IsError           bool            `json:"isError"`
	StructuredContent json.RawMessage `json:"structuredContent"`
	Content           []toolContent   `json:"content"`
}

type toolContent struct {
	Type string `json:"type"`
	Text string `json:"text"`
}

type profileSummary struct {
	ID       string `json:"id"`
	Title    string `json:"title"`
	IP       string `json:"ip"`
	Host     string `json:"host"`
	Protocol string `json:"protocol"`
}

type openSessionOutput struct {
	WorkspaceID string `json:"workspace_id"`
}

type sftpDirectoryOutput struct {
	Path    string             `json:"path"`
	Entries []sftpEntrySummary `json:"entries"`
}

type sftpEntrySummary struct {
	Name        string `json:"name"`
	Path        string `json:"path"`
	IsDirectory bool   `json:"is_directory"`
}

type toolOutcome struct {
	Tool       string
	HTTPStatus int
	Elapsed    time.Duration
	Payload    json.RawMessage
	ToolError  string
	RPCError   string
	Transport  string
}

type mcpClient struct {
	client   *http.Client
	endpoint string
	token    string
	timeout  time.Duration
	nextID   atomic.Int64
}

type scenarioRunner struct {
	api      *mcpClient
	logger   *log.Logger
	mu       sync.Mutex
	failures int
}

func newScenarioRunner(cfg scenarioConfig, scenario string) (*scenarioRunner, func(), error) {
	token, err := latestToken(cfg.logDir)
	if err != nil {
		return nil, nil, err
	}
	logFile, err := os.Create(filepath.Join(cfg.logDir, fmt.Sprintf(
		"mcp-%s-%s.log", scenario, time.Now().Format("20060102-150405.000"),
	)))
	if err != nil {
		return nil, nil, fmt.Errorf("create scenario log: %w", err)
	}
	logger := log.New(io.MultiWriter(os.Stdout, logFile), "", log.LstdFlags|log.Lmicroseconds)
	client := &mcpClient{
		client:   &http.Client{},
		endpoint: cfg.endpoint,
		token:    token,
		timeout:  cfg.timeout,
	}
	return &scenarioRunner{api: client, logger: logger}, func() { _ = logFile.Close() }, nil
}

func (runner *scenarioRunner) callTool(tool string, arguments map[string]any) toolOutcome {
	payload, status, elapsed, rpcErr, err := runner.api.call("tools/call", map[string]any{
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
	runner.logger.Printf(
		"mcp_tool tool=%s status=%d elapsed_ms=%d transport_error=%q rpc_error=%q tool_error=%q",
		tool,
		status,
		elapsed.Milliseconds(),
		outcome.Transport,
		outcome.RPCError,
		outcome.ToolError,
	)
	return outcome
}

func (runner *scenarioRunner) require(outcome toolOutcome) bool {
	if outcome.Transport == "" && outcome.RPCError == "" && outcome.ToolError == "" {
		return true
	}
	runner.mu.Lock()
	runner.failures++
	runner.mu.Unlock()
	runner.logger.Printf(
		"mcp_test_failure tool=%s transport_error=%q rpc_error=%q tool_error=%q",
		outcome.Tool,
		outcome.Transport,
		outcome.RPCError,
		outcome.ToolError,
	)
	return false
}

func (runner *scenarioRunner) selectProfile(cfg scenarioConfig) (profileSummary, error) {
	outcome := runner.callTool("list_profiles", map[string]any{})
	if !runner.require(outcome) {
		return profileSummary{}, errors.New("list_profiles failed")
	}
	payload, err := toolPayload(outcome.Payload)
	if err != nil {
		return profileSummary{}, fmt.Errorf("decode list_profiles: %w", err)
	}
	profiles := parseProfiles(payload)
	profile := chooseProfile(profiles, cfg.targetIP, cfg.profileID)
	if profile.ID == "" {
		return profileSummary{}, fmt.Errorf("no profile matched ip=%s profile_id=%s", cfg.targetIP, cfg.profileID)
	}
	runner.logger.Printf(
		"mcp_target_profile profile_id=%s title=%s ip=%s protocol=%s",
		profile.ID,
		profile.Title,
		profile.IP,
		profile.Protocol,
	)
	return profile, nil
}

func (runner *scenarioRunner) openSession(profile profileSummary, protocol string) (string, error) {
	outcome := runner.callTool("open_session", map[string]any{
		"profile_id": profile.ID,
		"ip":         profile.IP,
		"title":      profile.Title,
		"protocol":   protocol,
	})
	if !runner.require(outcome) {
		return "", fmt.Errorf("open_session(%s) failed", protocol)
	}
	payload, err := toolPayload(outcome.Payload)
	if err != nil {
		return "", fmt.Errorf("decode open_session(%s): %w", protocol, err)
	}
	var opened openSessionOutput
	if err := json.Unmarshal(payload, &opened); err != nil || opened.WorkspaceID == "" {
		return "", fmt.Errorf("invalid open_session(%s) output: %w", protocol, err)
	}
	runner.logger.Printf("mcp_workspace_opened protocol=%s workspace_id=%s", protocol, opened.WorkspaceID)
	return opened.WorkspaceID, nil
}

// MCP 暂不提供 close_session；保留脚本 helper，便于后续恢复。
/*
func (runner *scenarioRunner) closeSession(workspaceID string) {
	outcome := runner.callTool("close_session", map[string]any{"workspace_id": workspaceID})
	runner.require(outcome)
}
*/

func (runner *scenarioRunner) waitForSftp(workspaceID string, timeout time.Duration) (sftpDirectoryOutput, error) {
	deadline := time.Now().Add(timeout)
	for attempt := 1; ; attempt++ {
		outcome := runner.callTool("list_sftp_remote", map[string]any{"workspace_id": workspaceID})
		if runner.require(outcome) {
			payload, err := toolPayload(outcome.Payload)
			if err == nil {
				remote := parseSftpDirectory(payload)
				if remote.Path != "" {
					runner.logger.Printf(
						"mcp_sftp_remote_ready workspace_id=%s path=%s entries=%d attempts=%d",
						workspaceID,
						remote.Path,
						len(remote.Entries),
						attempt,
					)
					return remote, nil
				}
			}
		}
		if time.Now().After(deadline) {
			return sftpDirectoryOutput{}, fmt.Errorf("remote directory was not ready: workspace_id=%s", workspaceID)
		}
		time.Sleep(500 * time.Millisecond)
	}
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
	ctx, cancel := contextWithTimeout(api.timeout)
	defer cancel()
	request = request.WithContext(ctx)
	response, err := api.client.Do(request)
	elapsed := time.Since(startedAt)
	if err != nil {
		return nil, 0, elapsed, nil, classifyHTTPError(err)
	}
	defer response.Body.Close()
	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, response.StatusCode, elapsed, nil, fmt.Errorf("read response: %w", err)
	}
	decoded, err := decodeResponseBody(responseBody)
	if err != nil {
		return nil, response.StatusCode, elapsed, nil, err
	}
	var envelope rpcEnvelope
	if len(decoded) > 0 {
		if err := json.Unmarshal(decoded, &envelope); err != nil {
			return nil, response.StatusCode, elapsed, nil, fmt.Errorf("decode JSON-RPC response: %w", err)
		}
	}
	if response.StatusCode < http.StatusOK || response.StatusCode >= http.StatusMultipleChoices {
		return envelope.Result, response.StatusCode, elapsed, envelope.Error, fmt.Errorf("http %d", response.StatusCode)
	}
	return envelope.Result, response.StatusCode, elapsed, envelope.Error, nil
}

func contextWithTimeout(timeout time.Duration) (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), timeout)
}

func toolErrorText(result toolCallResult) string {
	for _, item := range result.Content {
		if item.Text != "" {
			return item.Text
		}
	}
	return "tool returned isError=true"
}

func toolPayload(raw json.RawMessage) (json.RawMessage, error) {
	var result toolCallResult
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, err
	}
	if len(result.StructuredContent) > 0 && string(result.StructuredContent) != "null" {
		return result.StructuredContent, nil
	}
	if len(result.Content) > 0 {
		return []byte(result.Content[0].Text), nil
	}
	return nil, errors.New("MCP tool result has no payload")
}

func parseProfiles(raw json.RawMessage) []profileSummary {
	var profiles []profileSummary
	_ = json.Unmarshal(raw, &profiles)
	return profiles
}

func chooseProfile(profiles []profileSummary, ip, profileID string) profileSummary {
	for _, profile := range profiles {
		if profileID != "" && profile.ID == profileID {
			return profile
		}
		if profileID == "" && (ip == "" || profile.IP == ip || profile.Host == ip) {
			return profile
		}
	}
	return profileSummary{}
}

func parseSftpDirectory(raw json.RawMessage) sftpDirectoryOutput {
	var directory sftpDirectoryOutput
	_ = json.Unmarshal(raw, &directory)
	return directory
}

func decodeResponseBody(body []byte) ([]byte, error) {
	trimmed := bytes.TrimSpace(body)
	if len(trimmed) == 0 {
		return nil, nil
	}
	if trimmed[0] == '{' || trimmed[0] == '[' {
		return trimmed, nil
	}
	var data []byte
	scanner := bufio.NewScanner(bytes.NewReader(trimmed))
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "data:") {
			data = []byte(strings.TrimSpace(strings.TrimPrefix(line, "data:")))
		}
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	if len(data) == 0 {
		return nil, fmt.Errorf("unsupported MCP response body: %q", compact(string(trimmed)))
	}
	return data, nil
}

func latestToken(logDir string) (string, error) {
	entries, err := os.ReadDir(logDir)
	if err != nil {
		return "", fmt.Errorf("read log directory: %w", err)
	}
	type candidate struct {
		path string
		date time.Time
	}
	var files []candidate
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".log" {
			continue
		}
		info, err := entry.Info()
		if err == nil {
			files = append(files, candidate{path: filepath.Join(logDir, entry.Name()), date: info.ModTime()})
		}
	}
	sort.Slice(files, func(i, j int) bool { return files[i].date.After(files[j].date) })
	pattern := regexp.MustCompile(`Agent MCP bearer token:\s+(\S+)`)
	for _, file := range files {
		handle, err := os.Open(file.path)
		if err != nil {
			continue
		}
		scanner := bufio.NewScanner(handle)
		var token string
		for scanner.Scan() {
			match := pattern.FindStringSubmatch(scanner.Text())
			if len(match) == 2 {
				token = match[1]
			}
		}
		_ = handle.Close()
		if token != "" {
			return token, nil
		}
	}
	return "", fmt.Errorf("MCP bearer token not found in %s", logDir)
}

func classifyHTTPError(err error) error {
	return err
}

func compact(value string) string {
	value = strings.Join(strings.Fields(value), " ")
	if len(value) > 500 {
		return value[:500] + "..."
	}
	return value
}
