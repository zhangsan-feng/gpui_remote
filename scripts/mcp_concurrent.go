package main

import (
	"flag"
	"fmt"
	"os"
	"sync"
)

func main() {
	cfg := scenarioConfig{}
	bindCommonFlags(&cfg)
	flag.IntVar(&cfg.rounds, "rounds", 4, "sequential rounds inside each concurrent workspace lane")
	flag.Parse()
	if cfg.rounds <= 0 {
		fmt.Fprintln(os.Stderr, "rounds must be positive")
		os.Exit(1)
	}

	runner, closeLog, err := newScenarioRunner(cfg, "concurrent")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer closeLog()

	runner.logger.Printf("mcp_concurrent_started endpoint=%s target_ip=%s rounds=%d", cfg.endpoint, cfg.targetIP, cfg.rounds)
	profile, err := runner.selectProfile(cfg)
	if err != nil {
		runner.failures++
		runner.logger.Printf("mcp_test_failure %v", err)
	} else {
		sshWorkspace, sshErr := runner.openSession(profile, "ssh")
		sftpWorkspace, sftpErr := runner.openSession(profile, "sftp")
		if sshErr != nil || sftpErr != nil {
			runner.failures++
			if sshErr != nil {
				runner.logger.Printf("mcp_test_failure %v", sshErr)
			}
			if sftpErr != nil {
				runner.logger.Printf("mcp_test_failure %v", sftpErr)
			}
			if sshWorkspace != "" {
				runner.closeSession(sshWorkspace)
			}
			if sftpWorkspace != "" {
				runner.closeSession(sftpWorkspace)
			}
		} else {
			if _, waitErr := runner.waitForSftp(sftpWorkspace, cfg.timeout); waitErr != nil {
				runner.failures++
				runner.logger.Printf("mcp_test_failure %v", waitErr)
			}
			runConcurrentReads(runner, sshWorkspace, sftpWorkspace, cfg.rounds)
			runner.closeSession(sftpWorkspace)
			runner.closeSession(sshWorkspace)
		}
	}
	runner.logger.Printf("mcp_concurrent_finished failures=%d", runner.failures)
	if runner.failures > 0 {
		os.Exit(1)
	}
}

func runConcurrentReads(runner *scenarioRunner, sshWorkspace, sftpWorkspace string, rounds int) {
	var waitGroup sync.WaitGroup
	waitGroup.Add(2)
	go func() {
		defer waitGroup.Done()
		for round := 1; round <= rounds; round++ {
			runner.require(runner.callTool("read_terminal", map[string]any{
				"workspace_id": sshWorkspace,
				"offset":       0,
				"limit":        20,
			}))
			runner.logger.Printf("mcp_concurrent_lane lane=ssh workspace_id=%s round=%d", sshWorkspace, round)
		}
	}()
	go func() {
		defer waitGroup.Done()
		for round := 1; round <= rounds; round++ {
			runner.require(runner.callTool("list_sftp_remote", map[string]any{
				"workspace_id": sftpWorkspace,
			}))
			runner.logger.Printf("mcp_concurrent_lane lane=sftp workspace_id=%s round=%d", sftpWorkspace, round)
		}
	}()
	waitGroup.Wait()
}
