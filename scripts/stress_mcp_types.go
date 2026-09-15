package main

import (
	"encoding/json"
	"log"
	"net/http"
	"regexp"
	"sync"
	"sync/atomic"
	"time"
)

const (
	targetIP        = "10.9.17.133"
	protocolVersion = "2025-03-26"
)

var tokenPattern = regexp.MustCompile(`Agent MCP bearer token:\s+(\S+)`)

var expectedTools = []string{
	"list_profiles",
	"open_session",
	"list_sftp_sessions",
	"list_sftp_local",
	"change_sftp_local_directory",
	"list_sftp_remote",
	"change_sftp_remote_directory",
	"upload_sftp",
	"download_sftp",
	"list_sftp_transfers",
	"watch_sftp_local",
	"stop_sftp_local_watch",
	"list_sftp_local_watches",
	"list_terminals",
	"read_terminal",
	"send_text",
	"send_key",
}

type config struct {
	requests          int
	workers           int
	connections       int
	concurrency       int
	rounds            int
	timeout           time.Duration
	endpoint          string
	logDir            string
	targetIP          string
	profileID         string
	stressOnly        bool
	disableKeepAlives bool
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
	Path    string            `json:"path"`
	Entries []sftpEntryOutput `json:"entries"`
}

type sftpEntryOutput struct {
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
	logger   *log.Logger
	nextID   atomic.Int64
}

type testSuite struct {
	api      *mcpClient
	cfg      config
	logger   *log.Logger
	mu       sync.Mutex
	failures []string
}
