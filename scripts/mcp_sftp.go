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

	runner, closeLog, err := newScenarioRunner(cfg, "sftp")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer closeLog()

	runner.logger.Printf("mcp_sftp_started endpoint=%s target_ip=%s", cfg.endpoint, cfg.targetIP)
	profile, err := runner.selectProfile(cfg)
	if err == nil {
		workspaceID, openErr := runner.openSession(profile, "sftp")
		if openErr != nil {
			runner.failures++
			runner.logger.Printf("mcp_test_failure %v", openErr)
		} else {
			if _, waitErr := runner.waitForSftp(workspaceID, cfg.timeout); waitErr != nil {
				runner.failures++
				runner.logger.Printf("mcp_test_failure %v", waitErr)
			}
			runner.closeSession(workspaceID)
		}
	} else {
		runner.failures++
		runner.logger.Printf("mcp_test_failure %v", err)
	}
	runner.logger.Printf("mcp_sftp_finished failures=%d", runner.failures)
	if runner.failures > 0 {
		os.Exit(1)
	}
}
