package main

import (
	"flag"
	"fmt"
	"os"
)

func main() {
	cfg := scenarioConfig{}
	bindCommonFlags(&cfg)
	flag.Parse()

	runner, closeLog, err := newScenarioRunner(cfg, "ssh")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer closeLog()

	runner.logger.Printf("mcp_ssh_started endpoint=%s target_ip=%s", cfg.endpoint, cfg.targetIP)
	profile, err := runner.selectProfile(cfg)
	if err == nil {
		workspaceID, openErr := runner.openSession(profile, "ssh")
		if openErr != nil {
			runner.failures++
			runner.logger.Printf("mcp_test_failure %v", openErr)
		} else {
			outcome := runner.callTool("read_terminal", map[string]any{
				"workspace_id": workspaceID,
				"offset":       0,
				"limit":        20,
			})
			runner.require(outcome)
			// MCP 暂不提供 close_session，保留调用示例以便后续恢复。
			// runner.closeSession(workspaceID)
		}
	} else {
		runner.failures++
		runner.logger.Printf("mcp_test_failure %v", err)
	}
	runner.logger.Printf("mcp_ssh_finished failures=%d", runner.failures)
	if runner.failures > 0 {
		os.Exit(1)
	}
}
