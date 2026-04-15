// cynic-coord — MCP STDIO proxy to the CYNIC kernel coordination API.
//
// Exposes coord/observe/health tools for any MCP-compatible agent
// (Claude Code, Gemini CLI, Codex CLI). Delegates to the running
// kernel over HTTP. Zero kernel coupling.
//
// Env:
//   CYNIC_REST_ADDR — kernel address (e.g. <TAILSCALE_CORE>:3030)
//   CYNIC_API_KEY   — bearer token
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

var (
	kernelAddr string
	apiKey     string
	httpClient = &http.Client{Timeout: 10 * time.Second}
)

func main() {
	kernelAddr = os.Getenv("CYNIC_REST_ADDR")
	apiKey = os.Getenv("CYNIC_API_KEY")
	if kernelAddr == "" {
		log.Fatal("CYNIC_REST_ADDR not set")
	}
	if !strings.HasPrefix(kernelAddr, "http") {
		kernelAddr = "http://" + kernelAddr
	}

	s := server.NewMCPServer(
		"cynic-coord",
		"0.1.0",
		server.WithToolCapabilities(false),
		server.WithRecovery(),
	)

	// ── Coordination tools ──
	s.AddTool(mcp.NewTool("cynic_coord_register",
		mcp.WithDescription("Register this agent with CYNIC. Call at session start."),
		mcp.WithString("agent_id", mcp.Required(), mcp.Description("Unique agent identifier")),
		mcp.WithString("intent", mcp.Required(), mcp.Description("What this agent is working on (1-500 chars)")),
		mcp.WithString("agent_type", mcp.Description("Agent type: claude-code, gemini-cli, codex-cli, etc.")),
	), handleRegister)

	s.AddTool(mcp.NewTool("cynic_coord_claim",
		mcp.WithDescription("Claim a file or zone before editing. Returns CONFLICT if another agent holds it."),
		mcp.WithString("agent_id", mcp.Required(), mcp.Description("Your agent ID")),
		mcp.WithString("target", mcp.Required(), mcp.Description("File path or zone to claim")),
		mcp.WithString("claim_type", mcp.Description("Type: file, zone, feature (default: file)")),
	), handleClaim)

	s.AddTool(mcp.NewTool("cynic_coord_release",
		mcp.WithDescription("Release claims. Omit target to release ALL your claims."),
		mcp.WithString("agent_id", mcp.Required(), mcp.Description("Your agent ID")),
		mcp.WithString("target", mcp.Description("Specific target to release, or omit for all")),
	), handleRelease)

	s.AddTool(mcp.NewTool("cynic_coord_who",
		mcp.WithDescription("Show active agents and their claims. Use before starting work."),
		mcp.WithString("agent_id", mcp.Description("Filter by specific agent")),
	), handleWho)

	// ── Observe tool ──
	s.AddTool(mcp.NewTool("cynic_observe",
		mcp.WithDescription("Record an observation (discovery, decision, blocker) in CYNIC's event log."),
		mcp.WithString("agent_id", mcp.Required(), mcp.Description("Your agent ID")),
		mcp.WithString("observation_type", mcp.Required(),
			mcp.Description("Type of observation"),
			mcp.Enum("discovery", "decision", "blocker", "handoff", "tool_use"),
		),
		mcp.WithString("content", mcp.Required(), mcp.Description("What was observed")),
		mcp.WithString("target", mcp.Description("File or component this relates to")),
	), handleObserve)

	// ── Health tool ──
	s.AddTool(mcp.NewTool("cynic_health",
		mcp.WithDescription("Check kernel health. No auth required."),
	), handleHealth)

	// ── Handoff tool ──
	s.AddTool(mcp.NewTool("cynic_handoff",
		mcp.WithDescription("Read or append to .handoff.md — semantic context for other agents."),
		mcp.WithString("action", mcp.Required(),
			mcp.Description("read or append"),
			mcp.Enum("read", "append"),
		),
		mcp.WithString("message", mcp.Description("Message to append (required for append action)")),
		mcp.WithString("agent_id", mcp.Description("Your agent ID (used in append header)")),
	), handleHandoff)

	if err := server.ServeStdio(s); err != nil {
		log.Fatalf("MCP server error: %v", err)
	}
}

// ── HTTP helpers ──

func kernelRequest(method, path string, body any) ([]byte, int, error) {
	var bodyReader io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, 0, err
		}
		bodyReader = bytes.NewReader(b)
	}

	req, err := http.NewRequest(method, kernelAddr+path, bodyReader)
	if err != nil {
		return nil, 0, err
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	if apiKey != "" {
		req.Header.Set("Authorization", "Bearer "+apiKey)
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, 0, fmt.Errorf("kernel unreachable: %w", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, resp.StatusCode, err
	}
	return data, resp.StatusCode, nil
}

func textResult(msg string) (*mcp.CallToolResult, error) {
	return mcp.NewToolResultText(msg), nil
}

func errorResult(msg string) (*mcp.CallToolResult, error) {
	return mcp.NewToolResultError(msg), nil
}

// ── Tool handlers ──

func handleRegister(_ context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	agentID, _ := req.RequireString("agent_id")
	intent, _ := req.RequireString("intent")
	agentType := "unknown"
	if t, err := req.RequireString("agent_type"); err == nil {
		agentType = t
	}

	body := map[string]string{
		"agent_id":   agentID,
		"intent":     intent,
		"agent_type": agentType,
	}

	data, status, err := kernelRequest("POST", "/coord/register", body)
	if err != nil {
		return errorResult(err.Error())
	}
	if status >= 400 {
		return errorResult(fmt.Sprintf("register failed (%d): %s", status, string(data)))
	}
	return textResult(fmt.Sprintf("Registered '%s' (%s). Intent: %s", agentID, agentType, intent))
}

func handleClaim(_ context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	agentID, _ := req.RequireString("agent_id")
	target, _ := req.RequireString("target")
	claimType := "file"
	if t, err := req.RequireString("claim_type"); err == nil {
		claimType = t
	}

	body := map[string]string{
		"agent_id":   agentID,
		"target":     target,
		"claim_type": claimType,
	}

	data, status, err := kernelRequest("POST", "/coord/claim", body)
	if err != nil {
		return errorResult(err.Error())
	}

	if status == 409 {
		return textResult(fmt.Sprintf("CONFLICT: '%s' is claimed by another agent. Details: %s", target, string(data)))
	}
	if status >= 400 {
		return errorResult(fmt.Sprintf("claim failed (%d): %s", status, string(data)))
	}
	return textResult(fmt.Sprintf("CLAIMED '%s' (%s) for '%s'", target, claimType, agentID))
}

func handleRelease(_ context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	agentID, _ := req.RequireString("agent_id")
	target := ""
	if t, err := req.RequireString("target"); err == nil {
		target = t
	}

	body := map[string]any{
		"agent_id": agentID,
	}
	if target != "" {
		body["target"] = target
	}

	data, status, err := kernelRequest("POST", "/coord/release", body)
	if err != nil {
		return errorResult(err.Error())
	}
	if status >= 400 {
		return errorResult(fmt.Sprintf("release failed (%d): %s", status, string(data)))
	}
	if target != "" {
		return textResult(fmt.Sprintf("Released '%s' for '%s'", target, agentID))
	}
	return textResult(fmt.Sprintf("Released ALL claims for '%s'. %s", agentID, string(data)))
}

func handleWho(_ context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	data, status, err := kernelRequest("GET", "/agents", nil)
	if err != nil {
		return errorResult(err.Error())
	}
	if status >= 400 {
		return errorResult(fmt.Sprintf("who failed (%d): %s", status, string(data)))
	}
	return textResult(string(data))
}

func handleObserve(_ context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	agentID, _ := req.RequireString("agent_id")
	obsType, _ := req.RequireString("observation_type")
	content, _ := req.RequireString("content")
	target := ""
	if t, err := req.RequireString("target"); err == nil {
		target = t
	}

	body := map[string]string{
		"agent_id":         agentID,
		"observation_type": obsType,
		"content":          content,
	}
	if target != "" {
		body["target"] = target
	}

	data, status, err := kernelRequest("POST", "/observe", body)
	if err != nil {
		return errorResult(err.Error())
	}
	if status >= 400 {
		return errorResult(fmt.Sprintf("observe failed (%d): %s", status, string(data)))
	}
	return textResult(fmt.Sprintf("Observed [%s] by '%s': %s", obsType, agentID, content))
}

func handleHealth(_ context.Context, _ mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	data, status, err := kernelRequest("GET", "/health", nil)
	if err != nil {
		return errorResult(fmt.Sprintf("kernel unreachable: %v", err))
	}
	return textResult(fmt.Sprintf("Status: %d\n%s", status, string(data)))
}

func handleHandoff(_ context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	action, _ := req.RequireString("action")
	handoffPath := ".handoff.md"

	switch action {
	case "read":
		data, err := os.ReadFile(handoffPath)
		if os.IsNotExist(err) {
			return textResult("No .handoff.md found. No prior context.")
		}
		if err != nil {
			return errorResult(fmt.Sprintf("read error: %v", err))
		}
		return textResult(string(data))

	case "append":
		message := ""
		if m, err := req.RequireString("message"); err == nil {
			message = m
		}
		if message == "" {
			return errorResult("message required for append")
		}
		agentID := "unknown"
		if a, err := req.RequireString("agent_id"); err == nil {
			agentID = a
		}

		entry := fmt.Sprintf("\n## [%s] %s\n%s\n",
			agentID, time.Now().UTC().Format("2006-01-02T15:04:05Z"), message)

		f, err := os.OpenFile(handoffPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
		if err != nil {
			return errorResult(fmt.Sprintf("write error: %v", err))
		}
		defer f.Close()
		if _, err := f.WriteString(entry); err != nil {
			return errorResult(fmt.Sprintf("write error: %v", err))
		}
		return textResult(fmt.Sprintf("Appended to .handoff.md by '%s'", agentID))

	default:
		return errorResult("action must be 'read' or 'append'")
	}
}
